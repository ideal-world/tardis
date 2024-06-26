use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use url::Url;

pub(crate) mod db;
pub use db::*;
pub(crate) mod web_server;
pub use web_server::*;
pub(crate) mod web_client;
pub use web_client::*;
pub(crate) mod cache;
pub use cache::*;
pub(crate) mod mq;
pub use mq::*;
pub(crate) mod search;
pub use search::*;
pub(crate) mod mail;
pub use mail::*;
pub(crate) mod os;
pub use os::*;

use crate::redact::Redact;

/// # Tardis Component Configuration
///
/// common structure for components with one default module and many submodules
///
/// - common: common config for all modules, default to `()`
/// - default: default module config
/// - modules: submodule configs
///
/// Common config should have a default value.
///
/// ## Construct from submodule config
/// For those common config has a default value, you can construct it from submodule config through `From` trait.
/// ```ignore
/// SomeConfig::from(submodule_config);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
pub struct TardisComponentConfig<T, C: Default = ()> {
    #[serde(flatten)]
    #[builder(default, setter(into))]
    /// common config for all modules
    common: C,
    #[serde(flatten)]
    /// the default module config
    pub default: T,
    #[builder(default, setter(into))]
    #[serde(default = "Default::default")]
    /// submodule configs
    pub modules: HashMap<String, T>,
}

impl<T, C: Default> std::ops::Deref for TardisComponentConfig<T, C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<T, C> Default for TardisComponentConfig<T, C>
where
    T: Default,
    C: Default,
{
    fn default() -> Self {
        Self {
            common: Default::default(),
            default: Default::default(),
            modules: HashMap::new(),
        }
    }
}

impl<T, C: Default> From<T> for TardisComponentConfig<T, C> {
    fn from(value: T) -> Self {
        Self {
            common: Default::default(),
            default: value,
            modules: HashMap::new(),
        }
    }
}

pub type DBConfig = TardisComponentConfig<db::DBModuleConfig>;

pub type CacheConfig = TardisComponentConfig<cache::CacheModuleConfig>;

pub type WebServerConfig = TardisComponentConfig<web_server::WebServerModuleConfig, web_server::WebServerCommonConfig>;

pub type WebClientConfig = TardisComponentConfig<web_client::WebClientModuleConfig>;

pub type MQConfig = TardisComponentConfig<mq::MQModuleConfig>;

pub type SearchConfig = TardisComponentConfig<search::SearchModuleConfig>;

pub type MailConfig = TardisComponentConfig<mail::MailModuleConfig>;

pub type OSConfig = TardisComponentConfig<os::OSModuleConfig>;

/// Advanced configuration / 高级配置
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct AdvConfig {
    /// Whether to capture the error stack / 是否捕捉错误堆栈
    ///
    /// Enable it to locate errors easily, but it will affect performance.
    ///
    /// 启用后可方便定位错误，但会影响性能.
    #[builder(default = false)]
    pub backtrace: bool,

    /// Configure field encryption salt value / 配置字段加密盐值
    ///
    /// Using the aes-ecb algorithm, salt consists of 16-bit English or numeric characters.
    ///
    /// Usage:
    /// . Open <https://www.javainuse.com/aesgenerator> and output the following:
    /// `Enter Plain Text to Encrypt ` = `Value to be encrypted` , `Select Mode` = `ECB` , `Key Size in Bits` = `128` , `Enter Secret Key` = `Value of this field` , `Output Text Format` = `Hex`
    /// . Click `Encrypt` to wrap the generated value in `ENC(xx)` to replace the original value
    #[builder(default)]
    pub salt: String,
}

#[derive(Serialize, Deserialize, Clone, TypedBuilder)]
pub struct ConfCenterConfig {
    #[builder(default = "nacos".to_string())]
    pub kind: String,
    pub url: Url,
    #[builder(default)]
    pub username: String,
    #[builder(default)]
    pub password: String,
    #[builder(default = Some("default".to_string()))]
    pub group: Option<String>,
    #[builder(default = Some("toml".to_string()))]
    pub format: Option<String>,
    #[builder(default)]
    pub namespace: Option<String>,
    #[builder(default = Some(30000), setter(strip_option))]
    /// config change polling interval, in milliseconds, default is 30000ms / 配置变更轮询间隔，单位毫秒, 默认30000ms
    pub config_change_polling_interval: Option<u64>,
}

impl std::fmt::Debug for ConfCenterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfCenterConfig")
            .field("kind", &self.kind)
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password", &self.password.redact())
            .field("group", &self.group)
            .field("format", &self.format)
            .field("namespace", &self.namespace)
            .field("config_change_polling_interval", &self.config_change_polling_interval)
            .finish()
    }
}
