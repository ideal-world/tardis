use super::*;

#[async_trait::async_trait]
pub(crate) trait Initializer {
    async fn init(&self, target: &TardisWebServer);
}

/// a tuple of (Code, WebServerModule) can be an initializer
#[async_trait::async_trait]
impl<T, MW, D> Initializer for (String, WebServerModule<T, MW, D>)
where
    T: Clone + OpenApi + 'static + Send + Sync,
    MW: Clone + Middleware<BoxEndpoint<'static>> + 'static + Send + Sync,
    D: Clone + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module) = self;
        let module_config = target.config.modules.get(code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {code} not found")).clone();
        target.do_add_module_with_data(code, &module_config, module.clone()).await;
    }
}

/// a tuple of (Code, WebServerModule, Config) can be an initializer, in this case we don't load config manually
#[async_trait::async_trait]
impl<T, MW, D> Initializer for (String, WebServerModule<T, MW, D>, WebServerModuleConfig)
where
    T: Clone + OpenApi + 'static + Send + Sync,
    MW: Clone + Middleware<BoxEndpoint<'static>> + 'static + Send + Sync,
    D: Clone + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module, module_config) = self;
        target.do_add_module_with_data(code, module_config, module.clone()).await;
    }
}

/// `TardisWebServer` itself can serve as an `Initializer`, it applies all of it's initializer to another
/// it will consume all initializer of stored previous webserver
#[async_trait::async_trait]
impl Initializer for TardisWebServer {
    #[inline]
    async fn init(&self, target: &TardisWebServer) {
        let mut target_initializers = target.initializers.lock().await;
        for i in self.initializers.lock().await.drain(..) {
            i.init(target).await;
            target_initializers.push(i);
        }
    }
}

/*
    gRPC support
*/
/// a tuple of (Code, WebServerModule) can be an initializer
#[cfg(feature = "web-server-grpc")]
#[async_trait::async_trait]
impl<T, MW, D> Initializer for (String, WebServerGrpcModule<T, MW, D>)
where
    T: Clone + poem::IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + poem_grpc::Service + 'static + Send + Sync,
    D: Clone + Send + Sync + 'static,
    MW: Clone + Middleware<BoxEndpoint<'static>> + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module) = self;
        let module_config = target.config.modules.get(code).unwrap_or_else(|| panic!("[Tardis.WebServer] Module {code} not found")).clone();
        target.do_add_grpc_module_with_data(code, &module_config, module.clone()).await;
    }
}

#[cfg(feature = "web-server-grpc")]
#[async_trait::async_trait]
impl<T, MW, D> Initializer for (String, WebServerGrpcModule<T, MW, D>, WebServerModuleConfig)
where
    T: Clone + poem::IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + poem_grpc::Service + 'static + Send + Sync,
    D: Clone + Send + Sync + 'static,
    MW: Clone + Middleware<BoxEndpoint<'static>> + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module, module_config) = self;
        target.do_add_grpc_module_with_data(code, module_config, module.clone()).await;
    }
}

/*
    loader methods
*/

impl TardisWebServer {
    /// Load an single initializer
    #[inline]
    pub(crate) async fn load_initializer(&self, initializer: impl Initializer + Send + Sync + 'static) {
        self.load_boxed_initializer(Box::new(initializer)).await;
    }

    /// Load an single boxed initializer
    pub(crate) async fn load_boxed_initializer(&self, initializer: Box<dyn Initializer + Send + Sync>) {
        initializer.init(self).await;
        self.initializers.lock().await.push(initializer);
    }
}
