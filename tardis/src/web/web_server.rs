use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::lock::Mutex;
use poem::endpoint::BoxEndpoint;
use poem::listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener};
use poem::middleware::Cors;
use poem::{EndpointExt, Middleware, Route};
use poem_openapi::{ExtraHeader, OpenApi, OpenApiService, ServerObject};

use tokio::time::Duration;
use tracing::{debug, info};

use crate::basic::result::TardisResult;
use crate::config::config_dto::{FrameworkConfig, WebServerConfig, WebServerModuleConfig};
use crate::web::uniform_error_mw::UniformError;
use crate::TardisFuns;

pub type BoxMiddleware<'a, T = BoxEndpoint<'a>> = Box<dyn Middleware<T, Output = T> + Send>;

pub struct TardisWebServer {
    app_name: String,
    version: String,
    config: WebServerConfig,
    initializers: Mutex<Vec<Box<dyn Initializer + Send>>>,
    route: Mutex<Route>,
}

#[async_trait::async_trait]
trait Initializer {
    async fn init(&self, target: &TardisWebServer);
}

#[derive(Clone)]
pub struct WebServerModule<T, MW = EmptyMiddleWare, D = String> {
    apis: T,
    data: Option<D>,
    middleware: MW,
}

#[async_trait::async_trait]
impl<T, MW, D> Initializer for (String, WebServerModule<T, MW, D>)
where
    T: Clone + OpenApi + 'static + Send + Sync ,
    MW: Clone + Middleware<BoxEndpoint<'static>> + 'static + Send + Sync ,
    D: Clone + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module) = self;
        let module_config = target.config.modules.get(code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {code} not found")).clone();
        target.do_add_module_with_data(code, &module_config, module.clone()).await;
    }
}

impl<T> WebServerModule<T> {
    pub fn new(apis: T) -> Self {
        Self {
            apis,
            data: None,
            middleware: EMPTY_MW,
        }
    }
}

impl<T, _MW, _D> WebServerModule<T, _MW, _D> {
    pub fn data<D>(self, data: D) -> WebServerModule<T, _MW, D> {
        WebServerModule {
            apis: self.apis,
            data: Some(data),
            middleware: self.middleware,
        }
    }
    pub fn middleware<MW>(self, middleware: MW) -> WebServerModule<T, MW, _D> {
        WebServerModule {
            apis: self.apis,
            data: self.data,
            middleware,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EmptyMiddleWare;
pub const EMPTY_MW: EmptyMiddleWare = EmptyMiddleWare;

impl Middleware<BoxEndpoint<'static>> for EmptyMiddleWare {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: BoxEndpoint<'static>) -> Self::Output {
        struct EmptyMiddleWareImpl<E>(E);

        #[async_trait::async_trait]
        impl<E: poem::Endpoint> poem::Endpoint for EmptyMiddleWareImpl<E> {
            type Output = poem::Response;

            async fn call(&self, req: poem::Request) -> poem::Result<Self::Output> {
                self.0.call(req).await.map(poem::IntoResponse::into_response)
            }
        }
        Box::new(EmptyMiddleWareImpl(ep))
    }
}

impl TardisWebServer {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            version: conf.app.version.clone(),
            config: conf.web_server.clone(),
            route: Mutex::default(),
            initializers: Mutex::new(Vec::new()),
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
            route: Mutex::default(),
            initializers: Mutex::new(Vec::new()),
        })
    }

    pub async fn add_route<T>(&self, apis: T) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_route_with_data::<_, String, EmptyMiddleWare>(apis, None, None).await
    }

    pub async fn add_route_with_ws<T>(&self, apis: T, capacity: usize) -> &Self
    where
        T: OpenApi + 'static,
    {
        self.add_route_with_data::<_, tokio::sync::broadcast::Sender<std::string::String>, EmptyMiddleWare>(apis, Some(tokio::sync::broadcast::channel::<String>(capacity).0), None)
            .await
    }

    pub async fn add_route_with_data<T, D, MW>(&self, apis: T, data: Option<D>, middlewares: Option<MW>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Middleware<BoxEndpoint<'static>> + Send + 'static,
    {
        let module = WebServerModuleConfig {
            name: self.app_name.clone(),
            version: self.version.clone(),
            doc_urls: self.config.doc_urls.clone(),
            req_headers: self.config.req_headers.clone(),
            ui_path: self.config.ui_path.clone(),
            spec_path: self.config.spec_path.clone(),
        };
        // self.do_add_module_with_data("", &module, apis, data, middlewares).await
        todo!()
    }

    pub async fn add_module<T, MW, D>(&self, code: &str, module: WebServerModule<T, MW, D>) -> &Self
    where
        T: Clone + Send + Sync + OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        self.add_module_with_data(code, module).await
    }

    pub async fn add_module_with_ws<T, MW>(&self, code: &str, apis: T, capacity: usize, middleware: Option<MW>) -> &Self
    where
        T: OpenApi + 'static,
        MW: Middleware<BoxEndpoint<'static>> + Send + 'static,
    {
        todo!();
        // self.add_module_with_data::<_, tokio::sync::broadcast::Sender<std::string::String>, EmptyMiddleWare>(code, apis, Some(tokio::sync::broadcast::channel::<String>(capacity).0), middlewares)
        //     .await
    }

    pub async fn add_module_with_data<T, D, MW>(&self, code: &str, module: WebServerModule<T, MW, D>) -> &Self
    where
        T: Clone + Send + Sync + OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        let code = code.to_lowercase();
        let code = code.as_str();
        let module_config = self.config.modules.get(code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {code} not found"));
        self.initializers.lock().await.push(Box::new((code.to_string(), module)));
        // self.do_add_module_with_data(code, module_config, module).await
        todo!()
    }

    async fn do_add_module_with_data<T, MW, D>(&self, code: &str, module_config: &WebServerModuleConfig, module: WebServerModule<T, MW, D>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Middleware<BoxEndpoint<'static>> + 'static,
    {
        info!("[Tardis.WebServer] Add module {}", code);
        let WebServerModule { apis, data, middleware } = module;
        let mut api_serv = OpenApiService::new(apis, &module_config.name, &module_config.version);
        for (env, url) in &module_config.doc_urls {
            let url = if !url.ends_with('/') { format!("{url}/{code}") } else { format!("{url}{code}") };
            api_serv = api_serv.server(ServerObject::new(url).description(env));
        }
        for (name, desc) in &module_config.req_headers {
            api_serv = api_serv.extra_request_header::<String, _>(ExtraHeader::new(name).description(desc));
        }
        let ui_serv = api_serv.rapidoc();
        let spec_serv = api_serv.spec_yaml();
        let mut route = Route::new();
        route = route.nest("/", api_serv);
        if let Some(ui_path) = &module_config.ui_path {
            route = route.nest(format!("/{ui_path}"), ui_serv);
        }
        if let Some(spec_path) = &module_config.spec_path {
            route = route.at(format!("/{spec_path}"), poem::endpoint::make_sync(move |_| spec_serv.clone()));
        }
        let cors = if &self.config.allowed_origin == "*" {
            // https://github.com/poem-web/poem/issues/161
            Cors::new()
        } else {
            Cors::new().allow_origin(&self.config.allowed_origin)
        };
        let mut route = route.boxed();
        let route = route.with(middleware);
        // let route = middleware.transform(route);
        // for middleware in middlewares {
        //     route = middleware.transform(route);
        // }
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
        // std::mem::swap(&mut swap_route, &mut *self.route.lock().await);
        // *self.route.lock().await = swap_route.nest(format!("/{code}"), route);
        self
    }
    pub async fn start(&self) -> TardisResult<()> {
        for initializer in self.initializers.lock().await.iter() {
            initializer.init(self).await;
        }
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
        let graceful_shutdown_signal = async move {
            let tardis_shut_down_signal = async {
                if let Some(mut rx) = TardisFuns::subscribe_shutdown_signal() {
                    match rx.recv().await {
                        Ok(_) => {}
                        Err(e) => {
                            debug!("[Tardis.WebServer] WebServer shutdown signal reciever got an error: {e}");
                        }
                    }
                } else {
                    futures::future::pending::<()>().await;
                }
            };
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    debug!("[Tardis.WebServer] WebServer shutdown (Crtl+C signal)");
                },
                _ = tardis_shut_down_signal => {
                    debug!("[Tardis.WebServer] WebServer shutdown (Tardis shutdown signal)");
                },
            };
        };
        if self.config.tls_key.is_some() {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port)).rustls(
                RustlsConfig::new().fallback(
                    RustlsCertificate::new()
                        .key(self.config.tls_key.clone().expect("[Tardis.WebServer] TLS key clone error"))
                        .cert(self.config.tls_cert.clone().expect("[Tardis.WebServer] TLS cert clone error")),
                ),
            );
            let server = poem::Server::new(bind).run_with_graceful_shutdown(swap_route, graceful_shutdown_signal, Some(Duration::from_secs(5)));
            info!("{}", output_info);
            server.await?;
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run_with_graceful_shutdown(swap_route, graceful_shutdown_signal, Some(Duration::from_secs(5)));
            info!("{}", output_info);
            server.await?;
        };
        Ok(())
    }
}
