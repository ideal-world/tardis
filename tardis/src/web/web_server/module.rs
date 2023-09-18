use poem::{endpoint::BoxEndpoint, Middleware};
use poem_openapi::OpenApi;
#[allow(unused_imports)]
#[doc = "for grpc"]
use std::sync::Arc;
use tokio::sync::broadcast;

/// Options for web server module
/// - uniform_error: whether to use uniform error response
#[derive(Clone)]
pub struct WebServerModuleOption {
    /// whether to use uniform error response
    pub uniform_error: bool,
}

impl WebServerModuleOption {
    pub fn set_uniform_error(&mut self, enable: bool) -> &mut Self {
        self.uniform_error = enable;
        self
    }
}

impl Default for WebServerModuleOption {
    fn default() -> Self {
        Self { uniform_error: true }
    }
}


/// A module of web server
#[derive(Clone)]
pub struct WebServerModule<T, MW = EmptyMiddleWare, D = ()> {
    /// A poem `Openapi` data structure
    pub apis: T,
    /// Shared data for this module
    pub data: Option<D>,
    /// Middleware for this module
    pub middleware: MW,
    /// Custom options for this module
    pub options: WebServerModuleOption,
}

impl<T, MW, D> Default for WebServerModule<T, MW, D>
where
    T: Default,
    MW: Default,
    D: Default,
{
    fn default() -> Self {
        Self {
            apis: Default::default(),
            data: Default::default(),
            middleware: Default::default(),
            options: Default::default(),
        }
    }
}

impl<T> From<T> for WebServerModule<T>
where
    T: OpenApi,
{
    fn from(apis: T) -> Self {
        WebServerModule::new(apis)
    }
}

impl<T, MW> From<(T, MW)> for WebServerModule<T, MW>
where
    MW: Middleware<BoxEndpoint<'static>>,
{
    fn from(value: (T, MW)) -> Self {
        let (apis, mw) = value;
        WebServerModule::new(apis).middleware(mw)
    }
}

impl<T, MW, D> From<(T, MW, D)> for WebServerModule<T, MW, D>
where
    MW: Middleware<BoxEndpoint<'static>>,
{
    fn from(value: (T, MW, D)) -> Self {
        let (apis, mw, data) = value;
        WebServerModule::new(apis).middleware(mw).data(data)
    }
}

impl<T> WebServerModule<T> {
    pub fn new(apis: T) -> Self {
        Self {
            apis,
            data: None,
            middleware: EmptyMiddleWare::INSTANCE,
            options: Default::default(),
        }
    }
}

impl<T, _MW, _D> WebServerModule<T, _MW, _D> {
    /// create a module with tokio broadcast sender as data
    /// ```ignore
    /// WebServerModule::from(MyApi).with_ws(100);
    /// ```
    pub fn with_ws(self, capacity: usize) -> WebServerModule<T, _MW, broadcast::Sender<String>> {
        WebServerModule {
            apis: self.apis,
            data: Some(broadcast::channel(capacity).0),
            options: self.options,
            middleware: self.middleware,
        }
    }

    pub fn data<D>(self, data: D) -> WebServerModule<T, _MW, D> {
        WebServerModule {
            apis: self.apis,
            data: Some(data),
            options: self.options,
            middleware: self.middleware,
        }
    }

    pub fn middleware<MW>(self, middleware: MW) -> WebServerModule<T, MW, _D> {
        WebServerModule {
            apis: self.apis,
            data: self.data,
            options: self.options,
            middleware,
        }
    }

    pub fn options(self, options: WebServerModuleOption) -> Self {
        WebServerModule { options, ..self }
    }
}

/// A middleware will do nothing
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct EmptyMiddleWare;

impl EmptyMiddleWare {
    pub const INSTANCE: Self = EmptyMiddleWare;
}

impl Middleware<BoxEndpoint<'static>> for EmptyMiddleWare {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: BoxEndpoint<'static>) -> Self::Output {
        // literally do nothing
        ep
    }
}

#[cfg(feature = "web-server-grpc")]
#[derive(Clone)]
pub struct WebServerGrpcModule<MW = EmptyMiddleWare, D = ()> {
    // pub web_server_module: WebServerModule<T, MW, D>,
    pub data: Option<D>,
    pub middleware: MW,
    pub(crate) grpc_router_mapper: Arc<dyn Fn(poem_grpc::RouteGrpc) -> poem_grpc::RouteGrpc + 'static + Sync + Send>,
    pub descriptor_sets: Vec<Vec<u8>>,
}

#[cfg(feature = "web-server-grpc")]
impl Default for WebServerGrpcModule {
    fn default() -> Self {
        Self {
            data: Default::default(),
            middleware: Default::default(),
            grpc_router_mapper: Arc::new(|route| route),
            descriptor_sets: vec![],
        }
    }
}

#[cfg(feature = "web-server-grpc")]
impl<_MW, _D> WebServerGrpcModule<_MW, _D> {
    pub fn data<D>(self, data: D) -> WebServerGrpcModule<_MW, D> {
        WebServerGrpcModule {
            data: Some(data),
            grpc_router_mapper: self.grpc_router_mapper,
            middleware: self.middleware,
            descriptor_sets: self.descriptor_sets,
        }
    }

    pub fn middleware<MW>(self, middleware: MW) -> WebServerGrpcModule<MW, _D> {
        WebServerGrpcModule {
            data: self.data,
            grpc_router_mapper: self.grpc_router_mapper,
            middleware,
            descriptor_sets: self.descriptor_sets,
        }
    }
}

#[cfg(feature = "web-server-grpc")]
impl<MW, D> WebServerGrpcModule<MW, D> {
    pub fn with_grpc_service<T: Clone>(mut self, service: T) -> Self
    where
        T: poem::IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + poem_grpc::Service + Send + Sync + 'static,
    {
        let previous_mapper = self.grpc_router_mapper;
        self.grpc_router_mapper = Arc::new(move |route| previous_mapper(route).add_service(service.clone()));
        self
    }
    pub fn with_descriptor(mut self, descriptor: Vec<u8>) -> Self {
        self.descriptor_sets.push(descriptor);
        self
    }
}

#[cfg(feature = "web-server-grpc")]
impl<T> From<T> for WebServerGrpcModule
where
    T: poem::IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + poem_grpc::Service + Clone + Send + Sync + 'static,
{
    fn from(api: T) -> Self {
        WebServerGrpcModule::default().with_grpc_service(api)
    }
}
