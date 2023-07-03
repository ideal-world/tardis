use std::fmt::Debug;

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
use crate::config::config_dto::{FrameworkConfig, WebServerConfig, WebServerModuleConfig};
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

pub struct TardisWebServer {
    app_name: String,
    version: String,
    config: WebServerConfig,
    /// Initializers here is **USED**, and being stored at here for next restart.
    ///
    /// Don't manually add initializer into here if you wan't `Initializer::init()` to be called,
    /// use `load_initializer` or `load_boxed_initializer` instead
    pub(self) initializers: Mutex<Vec<Box<dyn Initializer + Send + Sync>>>,
    state: Mutex<ServerState>,
}

impl TardisWebServer {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisWebServer> {
        TardisResult::Ok(TardisWebServer {
            app_name: conf.app.name.clone(),
            version: conf.app.version.clone(),
            config: conf.web_server.clone(),
            state: Mutex::default(),
            initializers: Mutex::new(Vec::new()),
        })
    }

    pub fn init_simple(host: &str, port: u16) -> TardisResult<TardisWebServer> {
        TardisResult::Ok(TardisWebServer {
            app_name: "".to_string(),
            version: "".to_string(),
            config: WebServerConfig {
                host: host.to_string(),
                port,
                ..Default::default()
            },
            state: Mutex::default(),
            initializers: Mutex::new(Vec::new()),
        })
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
        let module_config = WebServerModuleConfig {
            name: self.app_name.clone(),
            version: self.version.clone(),
            doc_urls: self.config.doc_urls.clone(),
            req_headers: self.config.req_headers.clone(),
            ui_path: self.config.ui_path.clone(),
            spec_path: self.config.spec_path.clone(),
            uniform_error: self.config.uniform_error,
        };
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
        let route = route.boxed();
        let route = route.with(middleware);
        if module_options.uniform_error || module_config.uniform_error {
            self.state.lock().await.add_route(code, route.with(UniformError).with(cors), data);
        } else {
            self.state.lock().await.add_route(code, route.with(cors), data);
        };
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
            host = self.config.host,
            port = self.config.port,
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
                    debug!("[Tardis.WebServer] WebServer shutdown (Crtl+C signal)");
                },
                _ = rx => {
                    debug!("[Tardis.WebServer] WebServer shutdown (Webserver shutdown signal)");
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
                Ok(())
            })
        } else {
            let bind = TcpListener::bind(format!("{}:{}", self.config.host, self.config.port));
            let server = poem::Server::new(bind).run_with_graceful_shutdown(route, graceful_shutdown_signal, Some(Duration::from_secs(5)));
            tokio::spawn(async {
                server.await?;
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
        if let ServerState::Running(task) = swap_state {
            info!("[Tardis.WebServer] Shutdown web server");
            let send_result = task.shutdown_trigger.send(());
            if send_result.is_err() {
                warn!("[Tardis.WebServer] Trying to shutdown webserver which seems already closed")
            };
            match task.inner.await {
                Ok(result) => return result,
                Err(e) => {
                    error!("[Tardis.WebServer] Fail to join webservert task: {e}")
                }
            }
        }
        drop(state_locked);
        Ok(())
    }
}

/// this await will pending until server is closed
impl std::future::Future for &TardisWebServer {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::Poll;
        let lock = self.state.lock();
        futures_util::pin_mut!(lock);
        cx.waker().wake_by_ref();
        match lock.poll(cx) {
            Poll::Ready(mut s) => {
                match &*s {
                    ServerState::Halted(_) => return Poll::Ready(()),
                    ServerState::Running(t) => {
                        if !t.inner.is_finished() {
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
