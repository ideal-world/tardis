use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct CacheModuleConfig {
    /// Cache access Url, Url with permission information / 缓存访问Url，Url带权限信息
    pub url: Url,
}
