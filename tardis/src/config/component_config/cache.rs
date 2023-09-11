use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;



#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct CacheModuleConfig {
    #[builder(default)]
    /// Cache access Url, Url with permission information / 缓存访问Url，Url带权限信息
    pub url: String,
}

impl Default for CacheModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}