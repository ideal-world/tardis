use crate::{
    serde::{Deserialize, Serialize},
    TardisFuns,
};
use serde_json::Value;
use std::collections::HashMap;

use typed_builder::TypedBuilder;
pub(crate) mod component;
pub(crate) mod log;
pub use component::*;
pub use log::*;
/// Configuration of Tardis / Tardis的配置
#[derive(Serialize, Deserialize, Clone, TypedBuilder, Debug)]
pub struct TardisConfig {
    #[builder(default, setter(into))]
    /// Project custom configuration / 项目自定义的配置
    pub cs: HashMap<String, Value>,
    #[builder(default)]
    /// Tardis framework configuration / Tardis框架的各功能配置
    pub fw: FrameworkConfig,
}

/// Configuration of each function of the Tardis framework / Tardis框架的各功能配置
#[derive(Debug, Serialize, Deserialize, Default, Clone, TypedBuilder)]
// TODO Replace with options / enums
#[builder(field_defaults(default, setter(strip_option, into)))]
#[serde(default)]
pub struct FrameworkConfig {
    #[builder(setter(!strip_option))]
    /// Application configuration / 应用配置
    pub app: AppConfig,
    #[builder(setter(!strip_option))]
    /// Advanced configuration / 高级配置
    pub adv: AdvConfig,
    /// Database configuration / 数据库配置
    pub db: Option<DBConfig>,
    /// Web service configuration / Web服务配置
    pub web_server: Option<WebServerConfig>,
    #[builder(default = Some(WebClientConfig::default()))]
    /// Web client configuration / Web客户端配置
    pub web_client: Option<WebClientConfig>,
    /// Distributed cache configuration / 分布式缓存配置
    pub cache: Option<CacheConfig>,
    /// Message queue configuration / 消息队列配置
    pub mq: Option<MQConfig>,
    /// Search configuration / 搜索配置
    pub search: Option<SearchConfig>,
    /// Mail configuration / 邮件配置
    pub mail: Option<MailConfig>,
    /// Object Storage configuration / 对象存储配置
    pub os: Option<OSConfig>,
    /// Config center configuration / 配置中心的配置
    #[cfg(feature = "conf-remote")]
    pub conf_center: Option<ConfCenterConfig>,
    /// log configuration / 日志配置
    pub log: Option<LogConfig>,
    /// Cluster configuration / 集群配置
    pub cluster: Option<ClusterConfig>,
}

impl FrameworkConfig {
    /// Get db config
    /// # Panic
    /// If the config of db is none, this will be panic.
    pub fn db(&self) -> &DBConfig {
        self.db.as_ref().expect("missing component config of db")
    }
    /// Get web_server config
    /// # Panic
    /// If the config of web_server is none, this will be panic.
    pub fn web_server(&self) -> &WebServerConfig {
        self.web_server.as_ref().expect("missing component config of web_server")
    }
    /// Get web_client config
    /// # Panic
    /// If the config of web_client is none, this will be panic.
    pub fn web_client(&self) -> &WebClientConfig {
        self.web_client.as_ref().expect("missing component config of web_client")
    }
    /// Get cache config
    /// # Panic
    /// If the config of cache is none, this will be panic.
    pub fn cache(&self) -> &CacheConfig {
        self.cache.as_ref().expect("missing component config of cache")
    }
    /// Get mq config
    /// # Panic
    /// If the config of mq is none, this will be panic.
    pub fn mq(&self) -> &MQConfig {
        self.mq.as_ref().expect("missing component config of mq")
    }
    /// Get search config
    /// # Panic
    /// If the config of search is none, this will be panic.
    pub fn search(&self) -> &SearchConfig {
        self.search.as_ref().expect("missing component config of search")
    }
    /// Get mail config
    /// # Panic
    /// If the config of mail is none, this will be panic.
    pub fn mail(&self) -> &MailConfig {
        self.mail.as_ref().expect("missing component config of mail")
    }
    /// Get os config
    /// # Panic
    /// If the config of os is none, this will be panic.
    pub fn os(&self) -> &OSConfig {
        self.os.as_ref().expect("missing component config of os")
    }
    /// Get log config
    /// # Panic
    /// If the config of log is none, this will be panic.
    pub fn log(&self) -> &LogConfig {
        self.log.as_ref().expect("missing component config of log")
    }
    /// Get cluster config
    /// # Panic
    /// If the config of cluster is none, this will be panic.
    pub fn cluster(&self) -> &ClusterConfig {
        self.cluster.as_ref().expect("missing component config of cluster")
    }
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
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct AppConfig {
    #[builder(default)]
    /// Application identifier / 应用标识
    ///
    /// Used to distinguish different services (applications) in a microservice environment.
    ///
    /// 在微服务环境下用于区别不同的服务（应用）.
    pub id: String,
    #[builder(default = String::from("Tardis Application"))]
    /// Application name / 应用名称
    pub name: String,
    #[builder(default = String::from("This is a Tardis Application"))]
    /// Application description / 应用描述
    pub desc: String,
    #[builder(default = String::from("0.0.1"))]
    /// Application version / 应用版本
    pub version: String,
    #[builder(default)]
    /// Application address / 应用地址
    ///
    /// Can be either the access address or the documentation address.
    ///
    /// 可以是访问地址，也可以是文档地址.
    pub url: String,
    #[builder(default)]
    /// Application contact email / 应用联系邮箱
    pub email: String,
    #[builder(default = format!("inst_{}", TardisFuns::field.nanoid()))]
    /// Application instance identification / 应用实例标识
    ///
    /// An application can have multiple instances, each with its own identity, using the nanoid by default.
    ///
    /// 一个应用可以有多个实例，每个实例都有自己的标识，默认使用nanoid.
    pub inst: String,
    #[builder(default, setter(strip_option))]
    /// Application default language / 应用默认语言
    /// https://www.andiamo.co.uk/resources/iso-language-codes/
    pub default_lang: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig::builder().build()
    }
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
    /// config change polling interval, in milliseconds / 配置变更轮询间隔，单位毫秒
    pub config_change_polling_interval: Option<u64>,
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
            config_change_polling_interval: Some(5000),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct ClusterConfig {
    pub watch_kind: String,
    #[cfg(feature = "k8s")]
    pub k8s_svc: Option<String>,
    pub cache_check_interval_sec: Option<i32>,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        #[cfg(feature = "k8s")]
        {
            ClusterConfig {
                watch_kind: "k8s".to_string(),
                k8s_svc: None,
                cache_check_interval_sec: None,
            }
        }
        #[cfg(not(feature = "k8s"))]
        {
            ClusterConfig {
                watch_kind: "cache".to_string(),
                cache_check_interval_sec: Some(10),
            }
        }
    }
}
