use std::env;
use std::fmt::Debug;
use std::path::Path;

use config::{Config, ConfigError, Environment, File};
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::basic::error::{TardisError, ERROR_DEFAULT_CODE};
use crate::basic::fetch_profile;
use crate::basic::result::TardisResult;
use crate::TardisFuns;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TardisConfig<T> {
    pub ws: T,
    pub fw: FrameworkConfig,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct FrameworkConfig {
    pub app: AppConfig,
    pub db: DBConfig,
    pub web_server: WebServerConfig,
    pub web_client: WebClientConfig,
    pub cache: CacheConfig,
    pub mq: MQConfig,
    pub adv: AdvConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub version: String,
    pub url: String,
    pub email: String,
    pub inst: String,
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
            inst: format!("inst_{}", TardisFuns::field.uuid()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DBConfig {
    pub enabled: bool,
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_sec: Option<u64>,
    pub idle_timeout_sec: Option<u64>,
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
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebServerConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub allowed_origin: String,
    pub context_flag: String,
    pub lang_flag: String,
    pub tls_key: Option<String>,
    pub tls_cert: Option<String>,
    pub modules: Vec<WebServerModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebServerModuleConfig {
    pub code: String,
    pub title: String,
    pub version: String,
    pub doc_urls: Vec<(String, String)>,
    // TODO
    pub authors: Vec<(String, String)>,
    pub ui_path: Option<String>,
    pub spec_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WebClientConfig {
    pub connect_timeout_sec: u64,
    pub request_timeout_sec: u64,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        WebServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 8080,
            allowed_origin: "*".to_string(),
            context_flag: "Tardis-Context".to_string(),
            lang_flag: "Accept-Language".to_string(),
            tls_key: None,
            tls_cert: None,
            modules: [WebServerModuleConfig::default()].to_vec(),
        }
    }
}

impl Default for WebServerModuleConfig {
    fn default() -> Self {
        WebServerModuleConfig {
            code: "".to_string(),
            title: "Tardis-based application".to_string(),
            version: "1.0.0".to_string(),
            doc_urls: [("test env".to_string(), "http://localhost:8080/".to_string())].to_vec(),
            authors: [("gudaoxuri".to_string(), "i@sunisle.org".to_string())].to_vec(),
            ui_path: Some("ui".to_string()),
            spec_path: Some("spec".to_string()),
        }
    }
}

impl Default for WebClientConfig {
    fn default() -> Self {
        WebClientConfig {
            connect_timeout_sec: 60,
            request_timeout_sec: 60,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub url: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            enabled: true,
            url: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct MQConfig {
    pub enabled: bool,
    pub url: String,
}

impl Default for MQConfig {
    fn default() -> Self {
        MQConfig {
            enabled: true,
            url: "".to_string(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AdvConfig {
    pub backtrace: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NoneConfig {}

impl<'a, T> TardisConfig<T>
where
    T: Deserialize<'a>,
{
    pub(crate) fn init(relative_path: &str) -> TardisResult<TardisConfig<T>> {
        let profile = fetch_profile();
        let path = Path::new(relative_path);

        let parent_path = env::current_dir().unwrap();

        info!(
            "[Tardis.Config] Initializing, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        let mut conf = Config::default();
        if !relative_path.is_empty() {
            conf.merge(File::from(path.join("conf-default")).required(true))?;
            conf.merge(File::from(Path::new(relative_path).join(&format!("conf-{}", profile))).required(true))?;
        }
        conf.merge(Environment::with_prefix("Tardis"))?;
        let workspace_config = conf.clone().try_into::<T>()?;
        let framework_config = conf.try_into::<FrameworkConfig>()?;

        env::set_var("RUST_BACKTRACE", if framework_config.adv.backtrace { "1" } else { "0" });

        info!(
            "[Tardis.Config] Initialized, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        debug!("=====[Tardis.Config] Content=====\n{:#?}\n=====", framework_config);

        Ok(TardisConfig {
            ws: workspace_config,
            fw: framework_config,
        })
    }
}

impl From<ConfigError> for TardisError {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::Frozen => TardisError::IOError(error.to_string()),
            ConfigError::NotFound(_) => TardisError::NotFound(error.to_string()),
            ConfigError::PathParse(_) => TardisError::IOError(error.to_string()),
            ConfigError::FileParse { .. } => TardisError::IOError(error.to_string()),
            ConfigError::Type { .. } => TardisError::FormatError(error.to_string()),
            ConfigError::Message(s) => TardisError::Custom(ERROR_DEFAULT_CODE.to_string(), s),
            ConfigError::Foreign(err) => TardisError::Box(err),
        }
    }
}
