use std::{collections::HashMap, sync::Arc};

use crate::{basic::result::TardisResult, config::config_dto::component::TardisComponentConfig};

#[async_trait::async_trait]
/// Initialize by config
///
/// This trait is used to initialize a struct by config. Which can be regarded as a async version of `From`.
pub trait InitBy<Initializer>: Sized {
    async fn init_by(initializer: &Initializer) -> TardisResult<Self>;
}

pub trait InitBySync<Initializer>: Sized
where
    Initializer: Sync,
{
    fn init_by(initializer: &Initializer) -> TardisResult<Self>;
}

#[async_trait::async_trait]
impl<Initializer, T: InitBySync<Initializer>> InitBy<Initializer> for T
where
    Initializer: Sync,
{
    /// Initialize by config
    async fn init_by(initializer: &Initializer) -> TardisResult<Self> {
        <Self as InitBySync<Initializer>>::init_by(initializer)
    }
}

#[async_trait::async_trait]
impl<ModuleConfig, CommonConfig: Default, Component> InitBy<TardisComponentConfig<ModuleConfig, CommonConfig>> for HashMap<String, Arc<Component>>
where
    Component: InitBy<ModuleConfig> + Sync + Send,
    CommonConfig: Sync + Send + 'static,
    ModuleConfig: Sync + Send + 'static,
{
    /// Initialize a hashmap of components by [`TardisComponentConfig`]
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
