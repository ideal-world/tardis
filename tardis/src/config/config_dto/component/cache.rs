use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

use crate::redact::Redact;
/// Distributed cache configuration / 分布式缓存配置
///
/// Distributed cache operations need to be enabled ```#[cfg(feature = "cache")]``` .
///
/// 分布式缓存操作需要启用 ```#[cfg(feature = "cache")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::CacheModuleConfig;
/// let config = CacheModuleConfig {
///    url: "redis://123456@127.0.0.1:6379".to_string(),
///    ..Default::default()
///};
/// ```
#[derive(Serialize, Deserialize, Clone, TypedBuilder)]
pub struct CacheModuleConfig {
    /// Cache access Url, Url with permission information / 缓存访问Url，Url带权限信息
    pub url: Url,
}

impl std::fmt::Debug for CacheModuleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheModuleConfig").field("url", &self.url.redact()).finish()
    }
}
