use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

use crate::redact::Redact;
/// Search configuration / 搜索配置
///
/// Search operation needs to be enabled ```#[cfg(feature = "web-client")]``` .
///
/// 搜索操作需要启用 ```#[cfg(feature = "web-client")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::SearchModuleConfig;
/// let config = SearchModuleConfig {
///    url: "https://elastic:123456@127.0.0.1:9200".parse().unwrap(),
///    ..Default::default()
///};
/// ```
#[derive(Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SearchModuleConfig {
    /// Search access Url, Url with permission information / 搜索访问Url，Url带权限信息
    pub url: Url,
    #[builder(default = 60)]
    /// Timeout / 操作超时时间
    pub timeout_sec: u64,
}

impl std::fmt::Debug for SearchModuleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchModuleConfig").field("url", &self.url.redact()).field("timeout_sec", &self.timeout_sec).finish()
    }
}
