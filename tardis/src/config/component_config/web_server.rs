use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use super::TardisComponentConfig;

/// Web service configuration / Web服务配置
///
/// Web service operations need to be enabled ```#[cfg(feature = "web-server")]``` .
///
/// Web服务操作需要启用 ```#[cfg(feature = "web-server")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::{WebServerConfig, WebServerModuleConfig};
/// let config = WebServerConfig {
///    modules: vec![
///        WebServerModuleConfig {
///            code: "todo".to_string(),
///            title: "todo app".to_string(),
///            doc_urls: [("test env".to_string(), web_url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())].iter().cloned().collect(),
///            ..Default::default()
///        },
///        WebServerModuleConfig {
///            code: "other".to_string(),
///            title: "other app".to_string(),
///            ..Default::default()
///        },
///    ],
///    tls_key: Some(TLS_KEY.to_string()),
///    tls_cert: Some(TLS_CERT.to_string()),
///    ..Default::default()
///};
/// ```

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
pub struct WebServerCommonConfig {
    #[builder(default = IpAddr::V4(Ipv4Addr::UNSPECIFIED), setter(into))]
    /// Web service Host, default is `0.0.0.0` / Web服务Host，默认为 `0.0.0.0`
    pub host: IpAddr,
    #[builder(default, setter(strip_option, into))]
    /// Directly accessible host, same as host by default / 可直接访问的host，默认与host相同
    pub access_host: Option<IpAddr>,
    #[builder(default = 8080)]
    /// Web service port, default is `8080` / Web服务端口，默认为 `8080`
    pub port: u16,
    #[builder(default, setter(strip_option))]
    /// Directly accessible port, same as port by default / 可直接访问的端口，默认与port相同
    pub access_port: Option<u16>,
    #[builder(default = String::from("*"), setter(into))]
    /// Allowed cross-domain sources, default is `*` / 允许的跨域来源，默认为 `*`
    pub allowed_origin: String,
    #[builder(default, setter(strip_option, into))]
    /// TLS Key, if this configuration is included then the protocol is HTTPS / TLS Key，如果包含此配置则协议为HTTPS
    pub tls_key: Option<String>,
    #[builder(default, setter(strip_option, into))]
    /// TLS certificate / TLS 证书
    pub tls_cert: Option<String>,
    #[builder(default)]
    /// Tardis context configuration / Tardis上下文配置
    pub context_conf: WebServerContextConfig,
}

/// Tardis context configuration / Tardis上下文配置
///
/// `Tardis Context` [TardisContext](crate::basic::dto::TardisContext) is used to bring in some
/// authentication information when a web request is received.
///
/// `Tardis上下文` [TardisContext](crate::basic::dto::TardisContext) 用于Web请求时带入一些认证信息.
///
/// This configuration specifies the source of the [TardisContext](crate::basic::dto::TardisContext).
///
/// 该配置用于指明 [TardisContext](crate::basic::dto::TardisContext) 的生成来源.
///
/// First it will try to get [context_header_name](Self::context_header_name) from the request header,
/// and if it is not specified or has no value it will try to get it from the cache.
///
/// 首先会尝试从请求头信息中获取 [context_header_name](Self::context_header_name) ,如果没指定或是没有值时会尝试从缓存中获取.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct WebServerContextConfig {
    /// Tardis context identifier, used to specify the request header name, default is `Tardis-Context`
    ///
    /// Tardis上下文标识，用于指定请求头名，默认为 `Tardis-Context`
    #[builder(default = String::from("Tardis-Context"), setter(into))]
    pub context_header_name: String,
    #[builder(default = String::from("tardis::ident::token::"), setter(into))]
    /// Tardis context identifier, used to specify the `key` of the cache, default is `tardis::ident::token::`
    ///
    /// Tardis上下文标识，用于指定缓存的 `key`，默认为 `tardis::ident::token::`
    pub token_cache_key: String,
}

/// Web module configuration / Web模块配置
///
/// An application can contain multiple web modules, each of which can have its own independent
/// request root path and API documentation.
///
/// 一个应用可以包含多个Web模块，每个模块可以有自己独立的请求根路径及API文档.
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::WebServerModuleConfig;
/// let config = WebServerModuleConfig {
///     code: "todo".to_string(),
///     title: "todo app".to_string(),
///     doc_urls: [
///         ("test env".to_string(), "http://127.0.0.1:8081".to_string()),
///         ("prod env".to_string(), "http://127.0.0.1:8082".to_string())
///     ].iter().cloned().collect(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct WebServerModuleConfig {
    #[builder(default = "Tardis-based application".to_string(), setter(into))]
    /// Module name for ``OpenAPI`` / 模块名称，用于 ``OpenAPI``
    pub name: String,
    #[builder(default = "1.0.0".to_string(), setter(into))]
    /// Module version for ``OpenAPI`` / 模块版本，用于 ``OpenAPI``
    pub version: String,
    #[builder(default = vec![("test env".to_string(), "http://localhost:8080/".to_string())], setter(into))]
    /// API request path for ``OpenAPI`` / API请求路径，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(environment identifier, request path)]`` / 格式为 ``[（环境标识，请求路径）]``
    pub doc_urls: Vec<(String, String)>,
    #[builder(default, setter(into))]
    /// Common request headers for ``OpenAPI`` / 公共请求头信息，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(header name, header description)]`` / 格式为 ``[（请求头名称，请求头说明）]``
    pub req_headers: Vec<(String, String)>,
    #[builder(default = Some(String::from("ui")), setter(strip_option, into))]
    /// ``OpenAPI`` UI path / 模``OpenAPI`` UI路径
    pub ui_path: Option<String>,
    #[builder(default = Some(String::from("spec")), setter(strip_option, into))]
    /// ``OpenAPI`` information path / ``OpenAPI`` 信息路径
    pub spec_path: Option<String>,
    #[builder(default = true)]
    /// Enable `UniformError` middleware / 启用 `UniformError` 中间件
    ///
    /// It's enabled by default. In some cases like running a mocker server, this may be supposed to be closed
    pub uniform_error: bool,
}

impl Default for WebServerContextConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Default for WebServerModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

