use crate::{
    serde::{Deserialize, Serialize},
    TardisFuns,
};
use serde_json::Value;
use std::collections::HashMap;
use super::component_config::*;
/// Configuration of Tardis / Tardis的配置
#[derive(Serialize, Deserialize, Clone)]
pub struct TardisConfig {
    /// Project custom configuration / 项目自定义的配置
    pub cs: HashMap<String, Value>,
    /// Tardis framework configuration / Tardis框架的各功能配置
    pub fw: FrameworkConfig,
}

/// Configuration of each function of the Tardis framework / Tardis框架的各功能配置
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
// TODO Replace with options / enums
pub struct FrameworkConfig {
    /// Application configuration / 应用配置
    pub app: AppConfig,
    /// Database configuration / 数据库配置
    pub db: Option<DBConfig>,
    /// Web service configuration / Web服务配置
    pub web_server: Option<WebServerConfig>,
    /// Web client configuration / Web客户端配置
    pub web_client: Option<WebClientConfig>,
    /// Distributed cache configuration / 分布式缓存配置
    pub cache: Option<CacheConfig>,
    /// Message queue configuration / 消息队列配置
    pub mq: Option<MqConfig>,
    /// Search configuration / 搜索配置
    pub search: Option<SearchConfig>,
    /// Mail configuration / 邮件配置
    pub mail: Option<MailConfig>,
    /// Object Storage configuration / 对象存储配置
    pub os: Option<OsConfig>,
    /// Advanced configuration / 高级配置
    pub adv: AdvConfig,
    /// Config center configuration / 配置中心的配置
    #[cfg(feature = "conf-remote")]
    pub conf_center: Option<ConfCenterConfig>,
    /// log configuration / 日志配置
    pub log: Option<LogConfig>,
    /// Cluster configuration / 集群配置
    pub cluster: Option<ClusterConfig>,
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

#[cfg(feature = "conf-remote")]
impl ConfCenterConfig {
    /// Reload configuration on remote configuration change / 远程配置变更时重新加载配置
    #[must_use]
    pub fn reload_on_remote_config_change(&self, relative_path: Option<&str>) -> tokio::sync::mpsc::Sender<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        let relative_path = relative_path.map(str::to_string);
        tokio::spawn(async move {
            match rx.recv().await {
                Some(_) => {}
                None => {
                    tracing::debug!("[Tardis.config] Configuration update channel closed");
                    return;
                }
            };
            if let Ok(config) = TardisConfig::init(relative_path.as_deref()).await {
                match TardisFuns::hot_reload(config).await {
                    Ok(_) => {
                        tracing::info!("[Tardis.config] Tardis hot reloaded");
                    }
                    Err(e) => {
                        tracing::error!("[Tardis.config] Tardis shutdown with error {}", e);
                    }
                }
            } else {
                tracing::error!("[Tardis.config] Configuration update failed: Failed to load configuration");
            }
            tracing::debug!("[Tardis.config] Configuration update listener closed")
        });
        tx
    }
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
pub struct LogConfig {
    pub level: String,
    #[cfg(feature = "tracing")]
    pub endpoint: String,
    #[cfg(feature = "tracing")]
    pub protocol: String,
    #[cfg(feature = "tracing")]
    pub server_name: String,
    #[cfg(feature = "tracing")]
    pub headers: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        #[cfg(feature = "tracing")]
        {
            LogConfig {
                level: "info".to_string(),
                endpoint: "http://localhost:4317".to_string(),
                protocol: "grpc".to_string(),
                server_name: "tardis-tracing".to_string(),
                headers: None,
            }
        }
        #[cfg(not(feature = "tracing"))]
        {
            LogConfig { level: "info".to_string() }
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
