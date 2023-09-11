use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
pub struct SearchModuleConfig {
    /// Search access Url, Url with permission information / 搜索访问Url，Url带权限信息
    pub url: Url,
    #[builder(default = 60)]
    /// Timeout / 操作超时时间
    pub timeout_sec: u64,
}