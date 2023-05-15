use crate::{
    serde::{Deserialize, Serialize},
    TardisFuns,
};
use serde_json::Value;
use std::collections::HashMap;

/// Configuration of Tardis / Tardis的配置
#[derive(Serialize, Clone)]
#[serde(default)]
pub struct TardisConfig {
    /// Project custom configuration / 项目自定义的配置
    pub cs: HashMap<String, Value>,
    /// Tardis framework configuration / Tardis框架的各功能配置
    pub fw: FrameworkConfig,
}

/// Configuration of each function of the Tardis framework / Tardis框架的各功能配置
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FrameworkConfig {
    /// Application configuration / 应用配置
    pub app: AppConfig,
    /// Database configuration / 数据库配置
    pub db: DBConfig,
    /// Web service configuration / Web服务配置
    pub web_server: WebServerConfig,
    /// Web client configuration / Web客户端配置
    pub web_client: WebClientConfig,
    /// Distributed cache configuration / 分布式缓存配置
    pub cache: CacheConfig,
    /// Message queue configuration / 消息队列配置
    pub mq: MQConfig,
    /// Search configuration / 搜索配置
    pub search: SearchConfig,
    /// Mail configuration / 邮件配置
    pub mail: MailConfig,
    /// Object Storage configuration / 对象存储配置
    pub os: OSConfig,
    /// Advanced configuration / 高级配置
    pub adv: AdvConfig,
    /// Config center configuration / 配置中心的配置
    #[cfg(feature = "conf-remote")]
    pub conf_center: Option<ConfCenterConfig>,
    /// Tracing configuration / 链路追踪配置
    #[cfg(feature = "tracing")]
    pub log: Option<LogConfig>,
}

/// Application configuration / 应用配置
///
/// By application, it means the current service
///
/// 所谓应用指的就是当前的服务
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::AppConfig;
/// AppConfig{
///     id: "todo".to_string(),
///     name: "Todo App".to_string(),
///     version: "1.0.0".to_string(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    /// Application identifier / 应用标识
    ///
    /// Used to distinguish different services (applications) in a microservice environment.
    ///
    /// 在微服务环境下用于区别不同的服务（应用）.
    pub id: String,
    /// Application name / 应用名称
    pub name: String,
    /// Application description / 应用描述
    pub desc: String,
    /// Application version / 应用版本
    pub version: String,
    /// Application address / 应用地址
    ///
    /// Can be either the access address or the documentation address.
    ///
    /// 可以是访问地址，也可以是文档地址.
    pub url: String,
    /// Application contact email / 应用联系邮箱
    pub email: String,
    /// Application instance identification / 应用实例标识
    ///
    /// An application can have multiple instances, each with its own identity, using the nanoid by default.
    ///
    /// 一个应用可以有多个实例，每个实例都有自己的标识，默认使用nanoid.
    pub inst: String,
    /// Application default language / 应用默认语言
    /// https://www.andiamo.co.uk/resources/iso-language-codes/
    pub default_lang: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            id: "".to_string(),
            name: "Tardis Application".to_string(),
            desc: "This is a Tardis Application".to_string(),
            version: "0.0.1".to_string(),
            url: "".to_string(),
            email: "".to_string(),
            inst: format!("inst_{}", TardisFuns::field.nanoid()),
            default_lang: None,
        }
    }
}

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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DBConfig {
    /// Whether to enable the database operation function / 是否启用数据库操作功能
    pub enabled: bool,
    /// Database access Url, Url with permission information / 数据库访问Url，Url带权限信息
    pub url: String,
    /// Maximum number of connections, default 20 / 最大连接数，默认 20
    pub max_connections: u32,
    /// Minimum number of connections, default 5 / 最小连接数，默认 5
    pub min_connections: u32,
    /// Connection timeout / 连接超时时间
    pub connect_timeout_sec: Option<u64>,
    /// Idle connection timeout / 空闲连接超时时间
    pub idle_timeout_sec: Option<u64>,
    /// Database module configuration / 数据库模块配置
    pub modules: HashMap<String, DBModuleConfig>,
    /// Compatible database type / 兼容数据库类型
    pub compatible_type: CompatibleType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DBModuleConfig {
    /// Database access Url, Url with permission information / 数据库访问Url，Url带权限信息
    pub url: String,
    /// Maximum number of connections, default 20 / 最大连接数，默认 20
    pub max_connections: u32,
    /// Minimum number of connections, default 5 / 最小连接数，默认 5
    pub min_connections: u32,
    /// Connection timeout / 连接超时时间
    pub connect_timeout_sec: Option<u64>,
    /// Idle connection timeout / 空闲连接超时时间
    pub idle_timeout_sec: Option<u64>,
    /// Compatible database type / 兼容数据库类型
    pub compatible_type: CompatibleType,
}

impl Default for DBConfig {
    fn default() -> Self {
        DBConfig {
            enabled: true,
            url: "".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_sec: None,
            idle_timeout_sec: None,
            modules: Default::default(),
            compatible_type: CompatibleType::None,
        }
    }
}

impl Default for DBModuleConfig {
    fn default() -> Self {
        DBModuleConfig {
            url: "".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_sec: None,
            idle_timeout_sec: None,
            compatible_type: CompatibleType::None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CompatibleType {
    None,
    Oracle,
}

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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebServerConfig {
    /// Whether to enable the web service operation function / 是否启用Web服务操作功能
    pub enabled: bool,
    /// Web service Host, default is `0.0.0.0` / Web服务Host，默认为 `0.0.0.0`
    pub host: String,
    /// Web service port, default is `8080` / Web服务端口，默认为 `8080`
    pub port: u16,
    /// Allowed cross-domain sources, default is `*` / 允许的跨域来源，默认为 `*`
    pub allowed_origin: String,
    /// TLS Key, if this configuration is included then the protocol is HTTPS / TLS Key，如果包含此配置则协议为HTTPS
    pub tls_key: Option<String>,
    /// TLS certificate / TLS 证书
    pub tls_cert: Option<String>,
    /// Whether to hide detailed error messages in the return message / 返回信息中是否隐藏详细错误信息
    pub security_hide_err_msg: bool,
    /// Tardis context configuration / Tardis上下文配置
    pub context_conf: WebServerContextConfig,
    /// API request path for ``OpenAPI`` / API请求路径，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(environment identifier, request path)]`` / 格式为 ``[（环境标识，请求路径）]``
    pub doc_urls: Vec<(String, String)>,
    /// Common request headers for ``OpenAPI`` / 公共请求头信息，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(header name, header description)]`` / 格式为 ``[（请求头名称，请求头说明）]``
    pub req_headers: Vec<(String, String)>,
    /// ``OpenAPI`` UI path / 模``OpenAPI`` UI路径
    pub ui_path: Option<String>,
    /// ``OpenAPI`` information path / ``OpenAPI`` 信息路径
    pub spec_path: Option<String>,
    /// Web module configuration / Web模块配置
    pub modules: HashMap<String, WebServerModuleConfig>,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebServerContextConfig {
    /// Tardis context identifier, used to specify the request header name, default is `Tardis-Context`
    ///
    /// Tardis上下文标识，用于指定请求头名，默认为 `Tardis-Context`
    pub context_header_name: String,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebServerModuleConfig {
    /// Module name for ``OpenAPI`` / 模块名称，用于 ``OpenAPI``
    pub name: String,
    /// Module version for ``OpenAPI`` / 模块版本，用于 ``OpenAPI``
    pub version: String,
    /// Module API request path for ``OpenAPI`` / 模块API请求路径，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(environment identifier, request path)]`` / 格式为 ``[（环境标识，请求路径）]``
    pub doc_urls: Vec<(String, String)>,
    /// Module common request headers for ``OpenAPI`` / 模块公共请求头信息，用于 ``OpenAPI``
    ///
    /// Formatted as ``[(header name, header description)]`` / 格式为 ``[（请求头名称，请求头说明）]``
    pub req_headers: Vec<(String, String)>,
    /// Module ``OpenAPI`` UI path / 模块 ``OpenAPI`` UI路径
    pub ui_path: Option<String>,
    /// Module ``OpenAPI`` information path / 模块 ``OpenAPI`` 信息路径
    pub spec_path: Option<String>,
}

impl Default for WebServerContextConfig {
    fn default() -> Self {
        WebServerContextConfig {
            context_header_name: "Tardis-Context".to_string(),
            token_cache_key: "tardis::ident::token::".to_string(),
        }
    }
}

impl Default for WebServerConfig {
    fn default() -> Self {
        WebServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 8080,
            allowed_origin: "*".to_string(),
            tls_key: None,
            tls_cert: None,
            security_hide_err_msg: false,
            context_conf: WebServerContextConfig::default(),
            doc_urls: [("test env".to_string(), "http://localhost:8080/".to_string())].to_vec(),
            req_headers: vec![],
            ui_path: Some("ui".to_string()),
            spec_path: Some("spec".to_string()),
            modules: Default::default(),
        }
    }
}

impl Default for WebServerModuleConfig {
    fn default() -> Self {
        WebServerModuleConfig {
            name: "Tardis-based application".to_string(),
            version: "1.0.0".to_string(),
            doc_urls: [("test env".to_string(), "http://localhost:8080/".to_string())].to_vec(),
            req_headers: vec![],
            ui_path: Some("ui".to_string()),
            spec_path: Some("spec".to_string()),
        }
    }
}

/// Web client configuration / Web客户端配置
///
/// Web client operation needs to be enabled ```#[cfg(feature = "web-client")]``` .
///
/// Web客户端操作需要启用 ```#[cfg(feature = "web-client")]``` .
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebClientConfig {
    /// Connection timeout / 连接超时时间
    pub connect_timeout_sec: u64,
    /// Request timeout / 请求超时时间
    pub request_timeout_sec: u64,
    /// Web client module configuration / Web客户端模块配置
    pub modules: HashMap<String, WebClientModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebClientModuleConfig {
    /// Connection timeout / 连接超时时间
    pub connect_timeout_sec: u64,
    /// Request timeout / 请求超时时间
    pub request_timeout_sec: u64,
}

impl Default for WebClientConfig {
    fn default() -> Self {
        WebClientConfig {
            connect_timeout_sec: 60,
            request_timeout_sec: 60,
            modules: Default::default(),
        }
    }
}

impl Default for WebClientModuleConfig {
    fn default() -> Self {
        WebClientModuleConfig {
            connect_timeout_sec: 60,
            request_timeout_sec: 60,
        }
    }
}

/// Distributed cache configuration / 分布式缓存配置
///
/// Distributed cache operations need to be enabled ```#[cfg(feature = "cache")]``` .
///
/// 分布式缓存操作需要启用 ```#[cfg(feature = "cache")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::CacheConfig;
/// let config = CacheConfig {
///    url: "redis://123456@127.0.0.1:6379".to_string(),
///    ..Default::default()
///};
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CacheConfig {
    /// Whether to enable the distributed cache operation function / 是否启用分布式缓存操作功能
    pub enabled: bool,
    /// Cache access Url, Url with permission information / 缓存访问Url，Url带权限信息
    pub url: String,
    /// Distributed cache module configuration / 分布式缓存模块配置
    pub modules: HashMap<String, CacheModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CacheModuleConfig {
    /// Cache access Url, Url with permission information / 缓存访问Url，Url带权限信息
    pub url: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            enabled: true,
            url: "".to_string(),
            modules: Default::default(),
        }
    }
}

impl Default for CacheModuleConfig {
    fn default() -> Self {
        CacheModuleConfig { url: "".to_string() }
    }
}

/// Message queue configuration / 消息队列配置
///
/// Message queue operation needs to be enabled ```#[cfg(feature = "mq")]``` .
///
/// 消息队列操作需要启用 ```#[cfg(feature = "mq")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::MQConfig;
/// let config = MQConfig {
///    url: "amqp://guest:guest@127.0.0.1:5672/%2f".to_string(),
///    ..Default::default()
///};
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MQConfig {
    /// Whether to enable the message queue operation function / 是否启用消息队列操作功能
    pub enabled: bool,
    /// Message queue access Url, Url with permission information / 消息队列访问Url，Url带权限信息
    pub url: String,
    /// Message queue module configuration / MQ模块配置
    pub modules: HashMap<String, MQModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MQModuleConfig {
    /// Message queue access Url, Url with permission information / 消息队列访问Url，Url带权限信息
    pub url: String,
}

impl Default for MQConfig {
    fn default() -> Self {
        MQConfig {
            enabled: true,
            url: "".to_string(),
            modules: Default::default(),
        }
    }
}

impl Default for MQModuleConfig {
    fn default() -> Self {
        MQModuleConfig { url: "".to_string() }
    }
}

/// Search configuration / 搜索配置
///
/// Search operation needs to be enabled ```#[cfg(feature = "web-client")]``` .
///
/// 搜索操作需要启用 ```#[cfg(feature = "web-client")]``` .
///
/// # Examples
/// ```ignore
/// use tardis::basic::config::ESConfig;
/// let config = ESConfig {
///    url: "https://elastic:123456@127.0.0.1:9200".to_string(),
///    ..Default::default()
///};
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchConfig {
    /// Whether to enable the search function / 是否启用搜索操作功能
    pub enabled: bool,
    /// Search access Url, Url with permission information / 搜索访问Url，Url带权限信息
    pub url: String,
    /// Timeout / 操作超时时间
    pub timeout_sec: u64,
    /// Search module configuration / 搜索模块配置
    pub modules: HashMap<String, SearchModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchModuleConfig {
    /// Search access Url, Url with permission information / 搜索访问Url，Url带权限信息
    pub url: String,
    /// Timeout / 操作超时时间
    pub timeout_sec: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            enabled: true,
            url: "".to_string(),
            timeout_sec: 60,
            modules: Default::default(),
        }
    }
}

impl Default for SearchModuleConfig {
    fn default() -> Self {
        SearchModuleConfig {
            url: "".to_string(),
            timeout_sec: 60,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MailConfig {
    /// Whether to enable the mail function / 是否启用邮件操作功能
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub default_from: String,
    /// Mail module configuration / 邮件模块配置
    pub modules: HashMap<String, MailModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MailModuleConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub default_from: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        MailConfig {
            enabled: true,
            smtp_host: "".to_string(),
            smtp_port: 0,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            default_from: "".to_string(),
            modules: Default::default(),
        }
    }
}

impl Default for MailModuleConfig {
    fn default() -> Self {
        MailModuleConfig {
            smtp_host: "".to_string(),
            smtp_port: 0,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            default_from: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct OSConfig {
    /// Whether to enable the object storage function / 是否启用对象存储操作功能
    pub enabled: bool,
    /// s3/oss/obs, Support amazon s3 / aliyun oss / huaweicloud obs
    pub kind: String,
    pub endpoint: String,
    pub ak: String,
    pub sk: String,
    pub region: String,
    pub default_bucket: String,
    /// Object Storage module configuration / 对象存储模块配置
    pub modules: HashMap<String, OSModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct OSModuleConfig {
    /// s3/oss/obs, Support amazon s3 / aliyun oss / huaweicloud obs
    pub kind: String,
    pub endpoint: String,
    pub ak: String,
    pub sk: String,
    pub region: String,
    pub default_bucket: String,
}

impl Default for OSConfig {
    fn default() -> Self {
        OSConfig {
            enabled: true,
            kind: "s3".to_string(),
            endpoint: "".to_string(),
            ak: "".to_string(),
            sk: "".to_string(),
            region: "".to_string(),
            default_bucket: "".to_string(),
            modules: Default::default(),
        }
    }
}

impl Default for OSModuleConfig {
    fn default() -> Self {
        OSModuleConfig {
            kind: "s3".to_string(),
            endpoint: "".to_string(),
            ak: "".to_string(),
            sk: "".to_string(),
            region: "".to_string(),
            default_bucket: "".to_string(),
        }
    }
}

/// Advanced configuration / 高级配置
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AdvConfig {
    /// Whether to capture the error stack / 是否捕捉错误堆栈
    ///
    /// Enable it to locate errors easily, but it will affect performance.
    ///
    /// 启用后可方便定位错误，但会影响性能.
    pub backtrace: bool,

    /// Configure field encryption salt value / 配置字段加密盐值
    ///
    /// Using the aes-ecb algorithm, salt consists of 16-bit English or numeric characters.
    ///
    /// Usage:
    /// . Open https://www.javainuse.com/aesgenerator and output the following:
    /// `Enter Plain Text to Encrypt ` = `Value to be encrypted` , `Select Mode` = `ECB` , `Key Size in Bits` = `128` , `Enter Secret Key` = `Value of this field` , `Output Text Format` = `Hex`
    /// . Click `Encrypt` to wrap the generated value in `ENC(xx)` to replace the original value
    pub salt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ConfCenterConfig {
    pub kind: String,
    pub url: String,
    pub username: String,
    pub password: String,
    pub group: Option<String>,
    pub format: Option<String>,
    pub namespace: Option<String>,
}

impl Default for ConfCenterConfig {
    fn default() -> Self {
        ConfCenterConfig {
            kind: "nacos".to_string(),
            url: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            format: Some("toml".to_string()),
            group: Some("default".to_string()),
            namespace: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LogConfig {
    pub level: String,
    pub endpoint: String,
    pub protocol: String,
    pub server_name: String,
    pub headers: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".to_string(),
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            server_name: "tardis-tracing".to_string(),
            headers: None,
        }
    }
}
