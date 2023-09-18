use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use url::Url;

use super::TardisComponentConfig;


/// Database configuration / 数据库配置
///
/// Database operations need to be enabled ```#[cfg(feature = "reldb")]``` .
///
/// 数据库的操作需要启用 ```#[cfg(feature = "reldb")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::DBConfig;
/// let config = DBConfig{
///    url: "mysql://root:123456@localhost:3306/test".to_string(),
///    ..Default::default()
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct DBModuleConfig {
    #[builder(setter(into))]
    /// Database access Url, Url with permission information / 数据库访问Url，Url带权限信息
    pub url: String,
    /// Maximum number of connections, default 20 / 最大连接数，默认 20
    #[builder(default = 20)]
    pub max_connections: u32,
    /// Minimum number of connections, default 5 / 最小连接数，默认 5
    #[builder(default = 5)]
    pub min_connections: u32,
    /// Connection timeout / 连接超时时间
    #[builder(default, setter(strip_option))]
    pub connect_timeout_sec: Option<u64>,
    /// Idle connection timeout / 空闲连接超时时间
    #[builder(default, setter(strip_option))]
    pub idle_timeout_sec: Option<u64>,
    /// Compatible database type / 兼容数据库类型
    #[builder(default)]
    pub compatible_type: CompatibleType,
}

impl Default for DBModuleConfig {
    fn default() -> Self {
        DBModuleConfig::builder().url("").build()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompatibleType {
    #[default]
    None,
    Oracle,
}
