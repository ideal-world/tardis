use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Web client configuration / Web客户端配置
///
/// Web client operation needs to be enabled ```#[cfg(feature = "web-client")]``` .
///
/// Web客户端操作需要启用 ```#[cfg(feature = "web-client")]``` .
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct WebClientModuleConfig {
    #[builder(default = 60, setter(into))]
    /// Connection timeout / 连接超时时间
    pub connect_timeout_sec: u64,
    #[builder(default = 60, setter(into))]
    /// Request timeout / 请求超时时间
    pub request_timeout_sec: u64,
    /// Whether to bypass TLS certificate validation (`danger_accept_invalid_certs`).
    ///
    /// Defaults to `false` so the client verifies server certificates by default.
    /// Setting this to `true` disables cert verification entirely and should only be
    /// used for local testing; it makes the client vulnerable to MITM attacks.
    ///
    /// 是否跳过 TLS 证书校验，默认 `false`。仅用于本地测试；开启后会受中间人攻击影响。
    #[builder(default = false)]
    pub allow_invalid_certs: bool,
}

impl Default for WebClientModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}
