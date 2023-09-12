use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{basic::result::TardisResult, config::config_dto::component_config::TardisComponentConfig};

use super::{TardisComponentMap, TardisComponentMapInner};

#[async_trait::async_trait]
pub trait InitBy<Initializer>: Sized {
    async fn init_by(initializer: &Initializer) -> TardisResult<Self>;
}

#[async_trait::async_trait]
impl<ModuleConfig, CommonConfig: Default, Component> InitBy<TardisComponentConfig<ModuleConfig, CommonConfig>> for HashMap<String, Arc<Component>>
where
    Component: InitBy<ModuleConfig> + Sync + Send,
    CommonConfig: Sync + Send + 'static,
    ModuleConfig: Sync + Send + 'static,
{
    async fn init_by(config: &TardisComponentConfig<ModuleConfig, CommonConfig>) -> TardisResult<Self> {
        let mut map = HashMap::new();
        let default_component = Component::init_by(&config.default).await?;
        map.insert(String::default(), Arc::new(default_component));
        for (code, module_config) in &config.modules {
            let module_component = Component::init_by(module_config).await?;
            map.insert(code.clone(), Arc::new(module_component));
        }
        Ok(map)
    }
}
