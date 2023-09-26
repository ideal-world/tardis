use std::sync::Arc;

use super::*;

#[async_trait::async_trait]
pub(crate) trait WebServerInitializer {
    async fn init(&self, target: &TardisWebServer);
}

/// a tuple of (Code, WebServerModule) can be an initializer
#[async_trait::async_trait]
impl<T, MW, D> WebServerInitializer for (String, WebServerModule<T, MW, D>)
where
    T: Clone + OpenApi + 'static + Send + Sync,
    MW: Clone + Middleware<BoxEndpoint<'static>> + 'static + Send + Sync,
    D: Clone + Send + Sync + 'static,
{
    async fn init(&self, target: &TardisWebServer) {
        let (code, ref module) = self;
        if let Some(module_config) = target.config.modules.get(code) {
            target.do_add_module_with_data(code, module_config, module.clone()).await;
        } else {
            crate::log::debug!("[Tardis.WebServer] Module {code} not found, using a default config.", code = code);
            target.do_add_module_with_data(code, &WebServerModuleConfig::default(), module.clone()).await;
        }
    }
}

/// a tuple of (Code, WebServerModule, Config) can be an initializer, in this case we don't load config manually
#[async_trait::async_trait]
impl<T, MW, D> WebServerInitializer for (String, WebServerModule<T, MW, D>, WebServerModuleConfig)
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
impl WebServerInitializer for TardisWebServer {
    #[inline]
    async fn init(&self, target: &TardisWebServer) {
        let mut target_initializers = target.initializers.lock().await;
        for i in self.initializers.lock().await.drain(..) {
            i.init(target).await;
            target_initializers.push(i);
        }
    }
}

/// `TardisWebServer` itself can serve as an `Initializer`, it applies all of it's initializer to another
/// it will consume all initializer of stored previous webserver
#[async_trait::async_trait]
impl WebServerInitializer for Arc<TardisWebServer> {
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
impl<MW, D> WebServerInitializer for (String, WebServerGrpcModule<MW, D>)
where
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
impl<MW, D> WebServerInitializer for (String, WebServerGrpcModule<MW, D>, WebServerModuleConfig)
where
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
    pub(crate) async fn load_initializer(&self, initializer: impl WebServerInitializer + Send + Sync + 'static) {
        self.load_boxed_initializer(Box::new(initializer)).await;
    }

    /// Load an single boxed initializer
    pub(crate) async fn load_boxed_initializer(&self, initializer: Box<dyn WebServerInitializer + Send + Sync>) {
        initializer.init(self).await;
        self.initializers.lock().await.push(initializer);
    }
}
