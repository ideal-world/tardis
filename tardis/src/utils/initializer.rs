use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{basic::result::TardisResult, config::component_config::TardisComponentConfig};

use super::{TardisComponentMap, TardisComponentMapInner};

#[async_trait::async_trait]
pub trait InitBy<Initializer>: Sized {
    async fn init(initializer: &Initializer) -> TardisResult<Self>;
}

#[async_trait::async_trait]
impl<ModuleConfig, CommonConfig, Component> InitBy<TardisComponentConfig<ModuleConfig, CommonConfig>> for HashMap<String, Arc<Component>>
where
    Component: InitBy<ModuleConfig> + Sync + Send,
    CommonConfig: Sync + Send + 'static,
    ModuleConfig: Sync + Send + 'static,
{
    async fn init(config: &TardisComponentConfig<ModuleConfig, CommonConfig>) -> TardisResult<Self> {
        let mut map = HashMap::new();
        let default_component = Component::init(&config.default).await?;
        map.insert(String::default(), Arc::new(default_component));
        for (code, module_config) in &config.modules {
            let module_component = Component::init(module_config).await?;
            map.insert(code.clone(), Arc::new(module_component));
        }
        Ok(map)
    }
}
