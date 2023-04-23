use config::builder::AsyncState;
use config::{ConfigBuilder, ConfigError, Environment, File};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
#[cfg(feature = "conf-remote")]
use {async_trait::async_trait, config::FileFormat, tokio::task::JoinHandle};

use crate::basic::error::TardisError;
use crate::basic::fetch_profile;
use crate::basic::locale::TardisLocale;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::log::{debug, info};

use super::config_dto::{ConfCenterConfig, TardisConfig};

/// Configuration handle / 配置处理
///
/// Organizing Configuration Management with Tardis Best Practices
///
/// 使用 Tardis 最佳实践组织配置管理
///
/// ## Configure fetch priority
///
/// 1. Local file: <local path>/conf-default.toml
/// 1. Local file: <local path>/conf-<profile>.toml
///     ``Requires [conf-remote] feature``
/// 1. Remote file: <fw.app.id>-default
///      ``Requires [conf-remote] feature``
/// 1. Remote file: <fw.app.id>-<profile>
/// 1. Environment variables starting with TARDIS
///
impl TardisConfig {
    pub(crate) async fn init(relative_path: Option<&str>) -> TardisResult<TardisConfig> {
        let profile = fetch_profile();
        let parent_path = env::current_dir().expect("[Tardis.Config] Current path get error");

        info!(
            "[Tardis.Config] Initializing, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );

        let config = TardisConfig::do_init(relative_path, &profile, None).await?;

        #[cfg(feature = "conf-remote")]
        let config = if let Some(conf_center) = &config.fw.conf_center {
            if config.fw.app.id.is_empty() {
                return Err(TardisError::format_error(
                    "[Tardis.Config] The [fw.app.id] must be set when the config center is enabled",
                    "",
                ));
            }
            TardisConfig::do_init(relative_path, &profile, Some((conf_center, &config.fw.app.id))).await?
        } else {
            config
        };

        info!(
            "[Tardis.Config] Initialized, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        debug!("=====[Tardis.Config] Content=====\n{:#?}\n=====", &config.fw);

        if let Some(relative_path) = relative_path {
            TardisLocale::init(Path::new(relative_path))?;
        }
        Ok(config)
    }

    async fn do_init(relative_path: Option<&str>, profile: &str, _conf_center: Option<(&ConfCenterConfig, &str)>) -> TardisResult<TardisConfig> {
        let mut conf = ConfigBuilder::<AsyncState>::default();

        // Fetch from local file
        if relative_path.is_some() {
            let path = Path::new(relative_path.unwrap_or(""));
            let file = path.join("conf-default");
            debug!("[Tardis.Config] Fetch local file: {:?}", file);
            conf = conf.add_source(File::from(file).required(true));
            if !profile.is_empty() {
                let file = path.join(format!("conf-{profile}").as_str());
                debug!("[Tardis.Config] Fetch local file: {:?}", file);
                conf = conf.add_source(File::from(file).required(true));
            }
        }

        #[cfg(feature = "conf-remote")]
        {
            // Fetch from remote
            if let Some((conf_center, app_id)) = _conf_center {
                let format = match conf_center.format.as_ref().unwrap_or(&"toml".to_string()).to_lowercase().as_str() {
                    "toml" => FileFormat::Toml,
                    "json" => FileFormat::Json,
                    "yaml" => FileFormat::Yaml,
                    _ => {
                        return Err(TardisError::format_error(
                            "[Tardis.Config] The file format of config center only supports [toml,json,yaml]",
                            "",
                        ))
                    }
                };
                info!(
                    "[Tardis.Config] Enabled config center: [{}] {} , start refetching configuration",
                    conf_center.kind, conf_center.url
                );
                match conf_center.kind.to_lowercase().as_str() {
                    "nacos" => {
                        let mut processor = crate::config::config_nacos::ConfNacosProcessor::init(
                            conf_center, 
                            profile, 
                            app_id, 
                            &Arc::new(format)
                        ).await?;
                        conf = processor.add_to_config(conf);
                        processor.watch();
                    },
                    _ => return Err(TardisError::format_error("[Tardis.Config] The kind of config center only supports [nacos]", "")),
                };
            }
        }

        // Fetch from ENV
        debug!("[Tardis.Config] Fetch env with prefix: TARDIS");
        conf = conf.add_source(Environment::with_prefix("TARDIS"));
        let conf = conf.build().await?;

        let mut workspace_config: HashMap<String, Value> = Default::default();
        match conf.get::<Value>("cs") {
            Ok(c) => {
                workspace_config.insert("".to_string(), c);
            }
            Err(error) => match error {
                ConfigError::NotFound(_) => {
                    info!("[Tardis.Config] No [cs] configuration found,use default configuration");
                }
                _ => return Err(error.into()),
            },
        }
        match conf.get::<HashMap<String, Value>>("csm") {
            Ok(c) => {
                workspace_config.extend(c);
            }
            Err(error) => match error {
                ConfigError::NotFound(_) => {
                    info!("[Tardis.Config] No [csm] configuration found,use default configuration");
                }
                _ => return Err(error.into()),
            },
        }
        let framework_config = match conf.get::<FrameworkConfig>("fw") {
            Ok(fw) => fw,
            Err(error) => match error {
                ConfigError::NotFound(_) => {
                    info!("[Tardis.Config] No [fw] configuration found,use default configuration");
                    FrameworkConfig::default()
                }
                _ => return Err(error.into()),
            },
        };

        env::set_var("RUST_BACKTRACE", if framework_config.adv.backtrace { "1" } else { "0" });

        let config = if framework_config.adv.salt.is_empty() {
            TardisConfig {
                cs: workspace_config,
                fw: framework_config,
            }
        } else {
            #[cfg(not(feature = "crypto"))]
            return Err(TardisError::format_error("[Tardis.Config] Configuration encryption must depend on the crypto feature", ""));
            #[cfg(feature = "crypto")]
            {
                // decryption processing
                let salt = framework_config.adv.salt.clone();
                let wc = decryption(&crate::TardisFuns::json.obj_to_string(&workspace_config)?, &salt)?;
                let fw = decryption(&crate::TardisFuns::json.obj_to_string(&framework_config)?, &salt)?;
                let workspace_config = crate::TardisFuns::json.str_to_obj(&wc)?;
                let framework_config = crate::TardisFuns::json.str_to_obj(&fw)?;
                TardisConfig {
                    cs: workspace_config,
                    fw: framework_config,
                }
            }
        };
        Ok(config)
    }
}

#[cfg(feature = "conf-remote")]
#[derive(std::fmt::Debug)]
pub(crate) struct HttpSource<F: config::Format> {
    pub processor: Box<dyn ConfCenterClient>,
    pub format: F,
    // pub md5: Arc<tokio::sync::Mutex<Option<String>>>,
}

#[async_trait]
pub trait ConfCenterClient: Sync + Send + std::fmt::Debug {
    async fn fetch(&mut self, format: FileFormat) -> TardisResult<config::Map<String, config::Value>>;
}

#[cfg(feature = "conf-remote")]
impl<F: config::Format> HttpSource<F> {
    pub(crate) fn new(processor: impl ConfCenterProcess, format: F) -> Self {
        todo!()
    }
}
pub(crate) trait ConfCenterProcessListener {
    fn has_changed(&self) -> bool;
    fn init() -> TardisResult<Self>
    where
        Self: Sized;
}

#[cfg(feature = "conf-remote")]
// temporarily dont need async_trait
// #[async_trait]
pub(crate) trait ConfCenterProcess: Sync + Send + std::fmt::Debug {
    type Source: config::AsyncSource + std::marker::Send + std::marker::Sync + 'static;
    fn get_sources(&mut self) -> Vec<Self::Source>;
    fn watch(self) -> JoinHandle<()>;
    fn add_to_config(&mut self, mut conf: ConfigBuilder<AsyncState>) -> ConfigBuilder<AsyncState> {
        for s in self.get_sources() {
            conf = conf.add_async_source(s);
        }
        conf
    }
}

#[cfg(feature = "conf-remote")]
#[async_trait]
impl<F> config::AsyncSource for HttpSource<F>
where
    F: config::Format + Send + Sync + std::fmt::Debug + 'static,
{
    async fn collect(&self) -> Result<config::Map<String, config::Value>, ConfigError> {
        todo!()
        // let response = reqwest::get(&self.url).await.map_err(|error| ConfigError::Foreign(Box::new(error)))?;
        // match response.status().as_u16() {
        //     404 => {
        //         log::warn!("[Tardis.Config] Fetch remote file: {} not found", &self.url);
        //         Ok(config::Map::default())
        //     }
        //     200 => {
        //         let config_text = response
        //         .text()
        //         .await
        //         .map_err(|error| ConfigError::Foreign(Box::new(error)))?;
        //         let mut md5 = crypto::md5::Md5::new();  
        //         md5.input_str(&config_text);
        //         let md5 = md5.result_str();
        //         {
        //             let mut md5_mutex = self.md5.lock().await;
        //             md5_mutex.replace(md5);
        //         }
        //         self.format.parse(Some(&self.url), &config_text).map_err(|error| ConfigError::Foreign(error))
        //     },
        //     _ => Err(ConfigError::Message(format!(
        //         "[Tardis.Config] Fetch remote file: {} error {}",
        //         &self.url,
        //         response.status().as_u16()
        //     ))),
        // }
    }
}

#[cfg(feature = "crypto")]
fn decryption(text: &str, salt: &str) -> TardisResult<String> {
    if salt.len() != 16 {
        return Err(TardisError::format_error("[Tardis.Config] [salt] Length must be 16", ""));
    }
    let enc_r = regex::Regex::new(r"(?P<ENC>ENC\([A-Za-z0-9+/]*\))")?;
    let text = enc_r
        .replace_all(text, |captures: &regex::Captures| {
            let data = captures.get(1).map_or("", |m| m.as_str()).to_string();
            let data = &data[4..data.len() - 1];
            crate::TardisFuns::crypto.aes.decrypt_ecb(data, salt).expect("[Tardis.Config] Decryption error")
        })
        .to_string();
    Ok(text)
}

impl From<ConfigError> for TardisError {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::Frozen => TardisError::io_error(&format!("[Tardis.Config] {error:?}"), "503-tardis-config-frozen"),
            ConfigError::NotFound(_) => TardisError::not_found(&format!("[Tardis.Config] {error:?}"), "404-tardis-config-not-exist"),
            ConfigError::PathParse(_) => TardisError::format_error(&format!("[Tardis.Config] {error:?}"), "406-tardis-config-parse-error"),
            ConfigError::FileParse { .. } => TardisError::format_error(&format!("[Tardis.Config] {error:?}"), "406-tardis-config-parse-error"),
            ConfigError::Type { .. } => TardisError::format_error(&format!("[Tardis.Config] {error:?}"), "406-tardis-config-parse-error"),
            ConfigError::Message(s) => TardisError::wrap(&format!("[Tardis.Config] {s:?}"), "-1-tardis-config-custom-error"),
            ConfigError::Foreign(error) => TardisError::wrap(&format!("[Tardis.Config] {error:?}"), "-1-tardis-config-error"),
        }
    }
}
