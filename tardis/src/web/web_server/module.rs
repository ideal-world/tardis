use poem::{endpoint::BoxEndpoint, Middleware};
use poem_openapi::OpenApi;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct WebServerModule<T, MW = EmptyMiddleWare, D = ()> {
    pub apis: T,
    pub data: Option<D>,
    pub middleware: MW,
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
            middleware: EMPTY_MW,
        }
    }
}

impl<T, _MW, _D> WebServerModule<T, _MW, _D> {
    /// create a module with tokio broadcast sender as data
    /// ```no_run
    /// WebServerModule::from(MyApi).with_ws(100);
    /// ```
    pub fn with_ws(self, capacity: usize) -> WebServerModule<T, _MW, broadcast::Sender<String>> {
        WebServerModule {
            apis: self.apis,
            data: Some(broadcast::channel(capacity).0),
            middleware: self.middleware,
        }
    }

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

/// A middleware will do nothing
#[derive(Debug, Copy, Clone)]
pub struct EmptyMiddleWare;

/// A middleware will do nothing
pub const EMPTY_MW: EmptyMiddleWare = EmptyMiddleWare;

impl Middleware<BoxEndpoint<'static>> for EmptyMiddleWare {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: BoxEndpoint<'static>) -> Self::Output {
        // literally do nothing
        ep
    }
}
