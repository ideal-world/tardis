use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

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
}

impl Default for WebClientModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}
