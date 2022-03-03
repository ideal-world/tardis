use poem::listener::{Listener, RustlsConfig, TcpListener};
use poem::middleware::Cors;
use poem::{EndpointExt, Route};
use poem_openapi::{OpenApi, OpenApiService, ServerObject};
use tokio::time::Duration;

use crate::basic::config::{FrameworkConfig, WebServerConfig};
use crate::basic::result::TardisResult;
use crate::log::info;
use crate::web::uniform_error_mw::UniformError;

pub struct TardisWebServer {
    app_name: String,
    config: WebServerConfig,
    rotue: Route,
}

impl TardisWebServer {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            config: conf.web_server.clone(),
            rotue: Route::new(),
        })
    }

    pub fn add_module<T>(&mut self, code: &str, apis: T) -> &mut Self
    where
        T: OpenApi + 'static,
    {
        self.add_module_with_data::<_, String>(code, apis, None)
    }

    pub fn add_module_with_data<T, D>(&mut self, code: &str, apis: T, data: Option<D>) -> &mut Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
    {
        let module = self.config.modules.iter().find(|m| m.code == code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {} not found", code));
        info!("[Tardis.WebServer] Add module {}", module.code);
        let mut api_serv = OpenApiService::new(apis, &module.title, &module.version);
        for (env, url) in &module.doc_urls {
            let url = if !url.ends_with('/') {
                format!("{}/{}", url, module.code)
            } else {
                format!("{}{}", url, module.code)
            };
            api_serv = api_serv.server(ServerObject::new(url).description(env));
        }
        let ui_serv = api_serv.rapidoc();
        let spec_serv = api_serv.spec();
        let mut route = Route::new();
        route = route.nest("/", api_serv);
        if let Some(ui_path) = &module.ui_path {
            route = route.nest(format!("/{}", ui_path), ui_serv);
        }
        if let Some(spec_path) = &module.spec_path {
            route = route.at(format!("/{}", spec_path), poem::endpoint::make_sync(move |_| spec_serv.clone()));
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
        std::mem::swap(&mut swap_route, &mut self.rotue);
        self.rotue = if let Some(data) = data {
            swap_route.nest(format!("/{}", code), route.data(data))
        } else {
            swap_route.nest(format!("/{}", code), route)
        };
        self
    }

    pub fn add_module_raw(&mut self, code: &str, route: Route) -> &mut Self {
        let mut swap_route = Route::new();
        std::mem::swap(&mut swap_route, &mut self.rotue);
        self.rotue = swap_route.nest(format!("/{}", code), route);
        self
    }

    pub async fn start(&'static self) -> TardisResult<()> {
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

        if self.config.tls_key.is_some() {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port)).rustls(
                RustlsConfig::new()
                    .key(self.config.tls_key.clone().expect("[Tardis.WebServer] TLS key clone error"))
                    .cert(self.config.tls_cert.clone().expect("[Tardis.WebServer] TLS cert clone error")),
            );
            let server = poem::Server::new(bind).run(&self.rotue);
            info!("{}", output_info);
            server.await?;
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run_with_graceful_shutdown(
                &self.rotue,
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
