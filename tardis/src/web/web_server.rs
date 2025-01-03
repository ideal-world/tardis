use std::fmt::Debug;

use std::net::IpAddr;
use std::sync::Arc;

use futures_util::lock::Mutex;
use poem::endpoint::BoxEndpoint;
use poem::listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener};
use poem::middleware::Cors;
use poem::{EndpointExt, Middleware, Route};
use poem_openapi::{ExtraHeader, OpenApi, OpenApiService, ServerObject};

use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use crate::basic::result::TardisResult;
use crate::config::config_dto::component::web_server::WebServerCommonConfig;
use crate::config::config_dto::{
    component::{web_server::WebServerModuleConfig, WebServerConfig},
    FrameworkConfig,
};
use crate::utils::initializer::InitBy;
use crate::web::uniform_error_mw::UniformError;
mod initializer;
use initializer::*;
mod module;
pub use module::*;
pub type BoxMiddleware<'a, T = BoxEndpoint<'a>> = Box<dyn Middleware<T, Output = T> + Send>;
type ServerTaskInner = JoinHandle<TardisResult<()>>;
struct ServerTask {
    pub(self) inner: ServerTaskInner,
    shutdown_trigger: oneshot::Sender<()>,
}

/// Server status hold by `TardisWebServer`
enum ServerState {
    /// ## Server is not running
    /// in that case, it hold route info
    Halted(Route),
    /// ## Server is running
    /// in that case, it hold join handle
    Running(ServerTask),
}

impl Debug for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted(_) => f.debug_tuple("Halted").finish(),
            Self::Running(_) => f.debug_tuple("Running").finish(),
        }
    }
}

impl ServerState {
    /// nest new route with optional data
    fn add_route<E, D>(&mut self, code: &str, route: E, data: Option<D>)
    where
        E: poem::IntoEndpoint,
        E::Endpoint: 'static,
        D: Clone + Send + Sync + 'static,
    {
        match self {
            ServerState::Halted(server_route) => {
                // Solved:  Cannot move out of *** which is behind a mutable reference
                // https://stackoverflow.com/questions/63353762/cannot-move-out-of-which-is-behind-a-mutable-reference
                let mut swap_route = Route::default();
                std::mem::swap(&mut swap_route, server_route);
                *server_route = if let Some(data) = data {
                    swap_route.nest(format!("/{code}"), route.data(data))
                } else {
                    swap_route.nest(format!("/{code}"), route)
                };
            }
            // if it is not halted, do nothing
            ServerState::Running(_) => {
                warn!("[Tardis.WebServer] Trying to add route to a running webserver, which won't make any change");
            }
        }
    }
    /// take out route, if it's running, return None
    fn take_route(&mut self) -> Option<Route> {
        match self {
            ServerState::Halted(route) => {
                let mut swap_route = Route::default();
                std::mem::swap(&mut swap_route, route);
                Some(swap_route)
            }
            ServerState::Running(_) => None,
        }
    }
}
impl Default for ServerState {
    fn default() -> Self {
        ServerState::Halted(Route::new())
    }
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ArcTardisWebServer(pub Arc<TardisWebServer>);

impl std::ops::Deref for ArcTardisWebServer {
    type Target = TardisWebServer;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl AsRef<TardisWebServer> for ArcTardisWebServer {
    fn as_ref(&self) -> &TardisWebServer {
        self.0.as_ref()
    }
}

impl From<Arc<TardisWebServer>> for ArcTardisWebServer {
    fn from(server: Arc<TardisWebServer>) -> Self {
        Self(server)
    }
}

#[derive(Debug)]
pub struct TardisWebServer {
    app_name: String,
    version: String,
    config: WebServerConfig,
    /// Initializers here is **USED**, and being stored at here for next restart.
    ///
    /// Don't manually add initializer into here if you wan't `Initializer::init()` to be called,
    /// use `load_initializer` or `load_boxed_initializer` instead
    pub(self) initializers: Mutex<Vec<Box<dyn WebServerInitializer + Send + Sync>>>,
    state: Mutex<ServerState>,
}

impl Default for TardisWebServer {
    fn default() -> Self {
        TardisWebServer {
            app_name: String::new(),
            version: String::new(),
            config: WebServerConfig::default(),
            state: Mutex::new(ServerState::default()),
            initializers: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl InitBy<FrameworkConfig> for TardisWebServer {
    async fn init_by(conf: &FrameworkConfig) -> TardisResult<Self> {
        let route = poem::Route::new();
        TardisResult::Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            version: conf.app.version.clone(),
            config: conf.web_server.clone().expect("missing web server config"),
            state: Mutex::new(ServerState::Halted(route)),
            initializers: Mutex::new(Vec::new()),
        })
    }
}
impl TardisWebServer {
    /// init a tardis webserver instance by framework config
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        let route = poem::Route::new();
        TardisResult::Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            version: conf.app.version.clone(),
            config: conf.web_server.clone().expect("missing web server config"),
            state: Mutex::new(ServerState::Halted(route)),
            initializers: Mutex::new(Vec::new()),
        })
    }

    /// init a simple server with host and port
    pub fn init_simple(host: IpAddr, port: u16) -> TardisResult<TardisWebServer> {
        let route = poem::Route::new();
        TardisResult::Ok(TardisWebServer {
            app_name: String::new(),
            version: String::new(),
            config: WebServerConfig::builder().common(WebServerCommonConfig::builder().host(host).port(port).build()).default(WebServerModuleConfig::builder().build()).build(),
            state: Mutex::new(ServerState::Halted(route)),
            initializers: Mutex::new(Vec::new()),
        })
    }

    /// get using default config
    pub fn get_default_config(&self) -> WebServerModuleConfig {
        WebServerModuleConfig {
            name: self.app_name.clone(),
            version: self.version.clone(),
            ..self.config.default.clone()
        }
    }
    /// add route
    /// # Usage
    /// ```ignore
    /// // add an api
    /// webserver.add_route(api).await;
    /// // add with middleware
    /// webserver.add_route((api, middleware)).await;
    /// // add with middleware and data
    /// webserver.add_route((api, middleware, data)).await;
    /// // add without middleware
    /// webserver.add_route((api, EmptyMiddleWare, data)).await;
    /// webserver.add_route(WebServerModule::from(api).data(data)).await;
    /// // add ws api
    /// webserver.add_route(WebServerModule::from(api).with_ws(ws_capacity)).await;
    /// // add with api, and custom options
    /// webserver.add_route(WebServerModule::from(api).data(data)).middleware(middleware).await;
    /// ```
    pub async fn add_route<T, D, MW>(&self, module: impl Into<WebServerModule<T, MW, D>>) -> &Self
    where
        T: Clone + Send + Sync + OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        let module_config = self.get_default_config();
        self.load_initializer(("".to_owned(), module.into(), module_config)).await;
        self
    }

    #[cfg(feature = "web-server-grpc")]
    pub async fn add_grpc_route<MW, D>(&self, module: impl Into<WebServerGrpcModule<MW, D>>) -> &Self
    where
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        let module_config = self.get_default_config();
        self.load_initializer(("".to_owned(), module.into(), module_config)).await;
        self
    }

    /// add an module
    /// # Usage
    /// refer method [`add_route()`](TardisWebServer::add_route)
    pub async fn add_module<T, MW, D>(&self, code: &str, module: impl Into<WebServerModule<T, MW, D>>) -> &Self
    where
        T: Clone + Send + Sync + OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        let code = code.to_lowercase();
        let code = code.as_str();
        self.load_initializer((code.to_string(), module.into())).await;
        self
    }

    #[cfg(feature = "web-server-grpc")]
    pub async fn add_grpc_module<MW, D>(&self, code: &str, module: impl Into<WebServerGrpcModule<MW, D>>) -> &Self
    where
        D: Clone + Send + Sync + 'static,
        MW: Clone + Send + Sync + Middleware<BoxEndpoint<'static>> + 'static,
    {
        let code = code.to_lowercase();
        let code = code.as_str();
        self.load_initializer((code.to_string(), module.into())).await;
        self
    }

    #[allow(unused_variables, unused_mut)]
    async fn do_add_module_with_data<T, MW, D>(&self, code: &str, module_config: &WebServerModuleConfig, module: WebServerModule<T, MW, D>) -> &Self
    where
        T: OpenApi + 'static,
        D: Clone + Send + Sync + 'static,
        MW: Middleware<BoxEndpoint<'static>> + 'static,
    {
        info!("[Tardis.WebServer] Add module {}", code);
        let WebServerModule {
            apis,
            data,
            middleware,
            options: module_options,
        } = module;
        let mut api_serv = OpenApiService::new(apis, &module_config.name, &module_config.version);
        for (env, url) in &module_config.doc_urls {
            let url = if !url.ends_with('/') { format!("{url}/{code}") } else { format!("{url}{code}") };
            api_serv = api_serv.server(ServerObject::new(url).description(env));
        }
        for (name, desc) in &module_config.req_headers {
            api_serv = api_serv.extra_request_header::<String, _>(ExtraHeader::new(name).description(desc));
        }
        let mut route = Route::new();
        #[allow(unused_assignments)]
        if let Some(ui_path) = &module_config.ui_path {
            let mut has_doc = false;
            #[cfg(feature = "openapi-redoc")]
            {
                let ui_serv = api_serv.redoc();
                route = route.nest(format!("/{ui_path}"), ui_serv);
                has_doc = true;
            }
            #[cfg(feature = "openapi-rapidoc")]
            {
                if !has_doc {
                    let ui_serv = api_serv.rapidoc();
                    route = route.nest(format!("/{ui_path}"), ui_serv);
                }
            }
            #[cfg(feature = "openapi-swagger")]
            {
                if !has_doc {
                    let ui_serv = api_serv.swagger_ui();
                    route = route.nest(format!("/{ui_path}"), ui_serv);
                }
            }
        }
        let spec_serv: String = api_serv.spec_yaml();
        if let Some(spec_path) = &module_config.spec_path {
            route = route.at(format!("/{spec_path}"), poem::endpoint::make_sync(move |_| spec_serv.clone()));
        }
        route = route.nest("/", api_serv);
        let cors = if &self.config.allowed_origin == "*" {
            // https://github.com/poem-web/poem/issues/161
            Cors::new()
        } else {
            Cors::new().allow_origin(&self.config.allowed_origin)
        };
        let route = route.boxed();
        let route = route.with(middleware).with(poem::middleware::Tracing).with(poem::middleware::CatchPanic::default());
        #[cfg(feature = "tracing")]
        let route = {
            let tracer = opentelemetry::global::tracer(crate::basic::tracing::tracing_service_name());
            route.with(poem::middleware::OpenTelemetryTracing::new(tracer))
        };
        if module_options.uniform_error || module_config.uniform_error {
            self.state.lock().await.add_route(code, route.with(UniformError).with(cors), data);
        } else {
            self.state.lock().await.add_route(code, route.with(cors), data);
        };
        self
    }

    #[cfg(feature = "web-server-grpc")]
    async fn do_add_grpc_module_with_data<MW, D>(&self, code: &str, _module_config: &WebServerModuleConfig, module: WebServerGrpcModule<MW, D>) -> &Self
    where
        D: Clone + Send + Sync + 'static,
        MW: Middleware<BoxEndpoint<'static>> + 'static,
    {
        use poem_grpc::RouteGrpc;
        info!("[Tardis.WebServer] Add grpc module {}", code);
        let WebServerGrpcModule {
            grpc_router_mapper,
            descriptor_sets,
            data,
            middleware,
        } = module;
        let mut route = grpc_router_mapper(RouteGrpc::new());
        let mut reflection = poem_grpc::Reflection::new();
        if descriptor_sets.is_empty() {
            warn!("[Tardis.WebServer] No descriptor set found for grpc module {}", code);
        } else {
            for descriptor in descriptor_sets {
                reflection = reflection.add_file_descriptor_set(&descriptor);
            }
        }
        route = route.add_service(reflection.build());
        route = route.add_service(poem_grpc::health_service().0);
        let route = route.with(poem::middleware::Tracing).boxed();
        let route = route.with(middleware);
        self.state.lock().await.add_route(code, route, data);
        self
    }

    /// # Warn
    /// Since `Route` didn't implement `Clone`, module create in this way cannot be reloaded while webserver restart
    pub async fn add_module_raw(&self, code: &str, route: Route) -> &Self {
        self.state.lock().await.add_route(code, route, Option::<()>::None);
        self
    }

    /// # Start
    /// Start this webserver
    ///
    /// to shutdown it by calling `TardisWebServer::shutdown()`
    pub async fn start(&self) -> TardisResult<()> {
        let output_info = format!(
            r#"
=================
[Tardis.WebServer] The {app} application has been launched. Visited at: {protocol}://{host}:{port}
================="#,
            app = self.app_name,
            host = self.config.access_host.as_ref().unwrap_or(&self.config.host),
            port = self.config.access_port.as_ref().unwrap_or(&self.config.port),
            protocol = if self.config.tls_key.is_some() { "https" } else { "http" }
        );

        // server_task will be locked until function return
        let mut state_locked = self.state.lock().await;
        let Some(route) = state_locked.take_route() else {
            // case of already running
            warn!("[Tardis.WebServer] Trying to start webserver while it is already running");
            return TardisResult::Ok(());
        };

        let (tx, rx) = oneshot::channel::<()>();
        let graceful_shutdown_signal = async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    debug!("[Tardis.WebServer] WebServer shutdown (Ctrl+C signal)");
                },
                _ = rx => {
                    debug!("[Tardis.WebServer] WebServer shutdown (WebServer shutdown signal)");
                },
            };
        };
        let boxed_server: ServerTaskInner = if self.config.tls_key.is_some() {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port)).rustls(
                RustlsConfig::new().fallback(
                    RustlsCertificate::new()
                        .key(self.config.tls_key.clone().expect("[Tardis.WebServer] TLS key clone error"))
                        .cert(self.config.tls_cert.clone().expect("[Tardis.WebServer] TLS cert clone error")),
                ),
            );
            let server = poem::Server::new(bind).run_with_graceful_shutdown(route, graceful_shutdown_signal, Some(Duration::from_secs(5)));
            tokio::spawn(async {
                server.await?;
                info!("[Tardis.WebServer] Poem webserver shutdown finished");
                Ok(())
            })
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run_with_graceful_shutdown(route, graceful_shutdown_signal, Some(Duration::from_secs(5)));
            tokio::spawn(async {
                server.await?;
                info!("[Tardis.WebServer] Poem webserver shutdown finished");
                Ok(())
            })
        };
        let task = ServerTask {
            inner: boxed_server,
            shutdown_trigger: tx,
        };
        *state_locked = ServerState::Running(task);
        drop(state_locked);
        info!("{}", output_info);
        TardisResult::Ok(())
    }

    /// # Shutdown
    /// shutdown this webserver, if it's not running it will return `Ok(())` instantly
    pub async fn shutdown(&self) -> TardisResult<()> {
        let mut state_locked = self.state.lock().await;
        let mut swap_state = ServerState::default();
        std::mem::swap(&mut *state_locked, &mut swap_state);
        drop(state_locked);
        if let ServerState::Running(task) = swap_state {
            info!("[Tardis.WebServer] Shutdown web server");
            let send_result = task.shutdown_trigger.send(());
            if send_result.is_err() {
                warn!("[Tardis.WebServer] Trying to shutdown webserver which seems already closed")
            };
            match tokio::time::timeout(Duration::from_secs(5), task.inner).await {
                Ok(Ok(result)) => return result,
                Ok(Err(e)) => {
                    error!("[Tardis.WebServer] Fail to join webservert task: {e}")
                }
                Err(e) => {
                    error!("[Tardis.WebServer] Shutdown webserver timeout: {e}")
                }
            }
        }
        Ok(())
    }

    /// return true if web server is running
    pub async fn is_running(&self) -> bool {
        let state = &*self.state.lock().await;
        matches!(state, ServerState::Running(t) if !t.inner.is_finished())
    }
}

/// this await will pending until server is closed
impl std::future::Future for &TardisWebServer {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::Poll;
        let lock = self.state.lock();
        const POLL_DURATION: Duration = Duration::from_millis(100);
        futures_util::pin_mut!(lock);
        match lock.poll(cx) {
            Poll::Ready(mut s) => {
                match &*s {
                    ServerState::Halted(_) => return Poll::Ready(()),
                    ServerState::Running(t) => {
                        if !t.inner.is_finished() {
                            let waker = cx.waker().clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(POLL_DURATION).await;
                                waker.wake();
                            });
                            return Poll::Pending;
                        }
                    }
                }
                *s = ServerState::default();
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl std::future::Future for ArcTardisWebServer {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let inner = self.0.as_ref();
        futures_util::pin_mut!(inner);
        inner.poll(cx)
    }
}
