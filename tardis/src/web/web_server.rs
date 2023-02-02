use futures_util::lock::Mutex;
use poem::listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener};
use poem::middleware::Cors;
use poem::{EndpointExt, Route};
use poem_openapi::{ExtraHeader, OpenApi, OpenApiService, ServerObject};
use tokio::time::Duration;

use crate::basic::result::TardisResult;
use crate::config::config_dto::{FrameworkConfig, WebServerConfig, WebServerModuleConfig};
use crate::log::info;
use crate::web::uniform_error_mw::UniformError;

pub struct TardisWebServer {
    app_name: String,
    version: String,
    config: WebServerConfig,
    route: Mutex<Route>,
}

impl TardisWebServer {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            version: conf.app.version.clone(),
            config: conf.web_server.clone(),
            route: Mutex::new(Route::new()),
        })
    }

    pub fn init_simple(host: &str, port: u16) -> TardisResult<TardisWebServer> {
        Ok(TardisWebServer {
            app_name: "".to_string(),
            version: "".to_string(),
            config: WebServerConfig {
                host: host.to_string(),
                port,
                ..Default::default()
            },
            route: Mutex::new(Route::new()),
        })
    }

    pub async fn add_route<T>(&self, apis: T) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_route_with_data::<_, String>(apis, None).await
    }

    pub async fn add_route_with_ws<T>(&self, apis: T, capacity: usize) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_route_with_data::<_, tokio::sync::broadcast::Sender<std::string::String>>(apis, Some(tokio::sync::broadcast::channel::<String>(capacity).0)).await
    }

    pub async fn add_route_with_data<T, D>(&self, apis: T, data: Option<D>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
    {
        let module = WebServerModuleConfig {
            name: self.app_name.clone(),
            version: self.version.clone(),
            doc_urls: self.config.doc_urls.clone(),
            req_headers: self.config.req_headers.clone(),
            ui_path: self.config.ui_path.clone(),
            spec_path: self.config.spec_path.clone(),
        };
        self.do_add_module_with_data("", &module, apis, data).await
    }

    pub async fn add_module<T>(&self, code: &str, apis: T) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_module_with_data::<_, String>(code, apis, None).await
    }

    pub async fn add_module_with_ws<T>(&self, code: &str, apis: T, capacity: usize) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_module_with_data::<_, tokio::sync::broadcast::Sender<std::string::String>>(code, apis, Some(tokio::sync::broadcast::channel::<String>(capacity).0)).await
    }

    pub async fn add_module_with_data<T, D>(&self, code: &str, apis: T, data: Option<D>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
    {
        let code = code.to_lowercase();
        let code = code.as_str();
        let module = self.config.modules.get(code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {code} not found"));
        self.do_add_module_with_data(code, module, apis, data).await
    }

    async fn do_add_module_with_data<T, D>(&self, code: &str, module: &WebServerModuleConfig, apis: T, data: Option<D>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
    {
        info!("[Tardis.WebServer] Add module {}", code);
        let mut api_serv = OpenApiService::new(apis, &module.name, &module.version);
        for (env, url) in &module.doc_urls {
            let url = if !url.ends_with('/') { format!("{url}/{code}") } else { format!("{url}{code}") };
            api_serv = api_serv.server(ServerObject::new(url).description(env));
        }
        for (name, desc) in &module.req_headers {
            api_serv = api_serv.extra_request_header::<String, _>(ExtraHeader::new(name).description(desc));
        }
        let ui_serv = api_serv.rapidoc();
        let spec_serv = api_serv.spec();
        let mut route = Route::new();
        route = route.nest("/", api_serv);
        if let Some(ui_path) = &module.ui_path {
            route = route.nest(format!("/{ui_path}"), ui_serv);
        }
        if let Some(spec_path) = &module.spec_path {
            route = route.at(format!("/{spec_path}"), poem::endpoint::make_sync(move |_| spec_serv.clone()));
        }
        let cors = if &self.config.allowed_origin == "*" {
            // https://github.com/poem-web/poem/issues/161
            Cors::new()
        } else {
            Cors::new().allow_origin(&self.config.allowed_origin)
        };
        let route = route.with(UniformError).with(cors);
        // Solved:  Cannot move out of *** which is behind a mutable reference
        // https://stackoverflow.com/questions/63353762/cannot-move-out-of-which-is-behind-a-mutable-reference
        let mut swap_route = Route::new();
        std::mem::swap(&mut swap_route, &mut *self.route.lock().await);
        *self.route.lock().await = if let Some(data) = data {
            swap_route.nest(format!("/{code}"), route.data(data))
        } else {
            swap_route.nest(format!("/{code}"), route)
        };
        self
    }

    pub async fn add_module_raw(&self, code: &str, route: Route) -> &Self {
        let mut swap_route = Route::new();
        std::mem::swap(&mut swap_route, &mut *self.route.lock().await);
        *self.route.lock().await = swap_route.nest(format!("/{code}"), route);
        self
    }

    pub async fn start(&self) -> TardisResult<()> {
        let output_info = format!(
            r#"
=================
[Tardis.WebServer] The {app} application has been launched. Visited at: {protocol}://{host}:{port}
================="#,
            app = self.app_name,
            host = self.config.host,
            port = self.config.port,
            protocol = if self.config.tls_key.is_some() { "https" } else { "http" }
        );

        let mut swap_route = Route::new();
        std::mem::swap(&mut swap_route, &mut *self.route.lock().await);
        if self.config.tls_key.is_some() {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port)).rustls(
                RustlsConfig::new().fallback(
                    RustlsCertificate::new()
                        .key(self.config.tls_key.clone().expect("[Tardis.WebServer] TLS key clone error"))
                        .cert(self.config.tls_cert.clone().expect("[Tardis.WebServer] TLS cert clone error")),
                ),
            );
            let server = poem::Server::new(bind).run_with_graceful_shutdown(
                swap_route,
                async move {
                    let _ = tokio::signal::ctrl_c().await;
                },
                Some(Duration::from_secs(5)),
            );
            info!("{}", output_info);
            server.await?;
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run_with_graceful_shutdown(
                swap_route,
                async move {
                    let _ = tokio::signal::ctrl_c().await;
                },
                Some(Duration::from_secs(5)),
            );
            info!("{}", output_info);
            server.await?;
        };
        Ok(())
    }
}
