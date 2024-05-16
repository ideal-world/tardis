use config::builder::AsyncState;
use config::{ConfigBuilder, ConfigError, Environment, File};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::Path;
#[cfg(feature = "conf-remote")]
use {config::FileFormat, std::sync::Arc};

use crate::basic::error::TardisError;
use crate::basic::fetch_profile;
use crate::basic::locale::TardisLocale;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use tracing::{debug, info};

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
                // listen reload signal
                let reload_notifier = conf_center.reload_on_remote_config_change(relative_path);
                let processor: Box<dyn ConfCenterProcess> = match conf_center.kind.to_lowercase().as_str() {
                    "nacos" => Box::new(crate::config::config_nacos::ConfNacosProcessor::init(conf_center, profile, app_id, &Arc::new(format)).await?),
                    _ => return Err(TardisError::format_error("[Tardis.Config] The kind of config center only supports [nacos]", "")),
                };
                conf = processor.register_to_config(conf);
                // listen update, if update, send reload signal
                processor.listen_update(&reload_notifier);
            }
        }

        // Fetch from ENV
        debug!("[Tardis.Config] Fetch env with prefix: TARDIS");
        conf = conf.add_source(Environment::with_prefix("TARDIS"));
        let conf = conf.build().await?;

        let mut workspace_config: HashMap<String, Value> = Default::default();
        match conf.get::<Value>("cs") {
            Ok(c) => {
                workspace_config.insert(String::new(), c);
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
// temporarily don't need async_trait
// #[async_trait]
pub(crate) trait ConfCenterProcess: Sync + Send + std::fmt::Debug {
    /// listen the config-center processor change
    fn listen_update(&self, reload_notifier: &tokio::sync::mpsc::Sender<()>);
    /// Add all sources to config
    fn register_to_config(&self, conf: ConfigBuilder<AsyncState>) -> ConfigBuilder<AsyncState>;
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
                match crate::TardisFuns::hot_reload(config).await {
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

#[cfg(feature = "crypto")]
pub fn decryption(text: &str, salt: &str) -> TardisResult<String> {
    use crate::crypto::crypto_aead::algorithm::Aes128;
    if salt.len() != 16 {
        return Err(TardisError::format_error("[Tardis.Config] [salt] Length must be 16", ""));
    }
    let enc_r = regex::Regex::new(r"(?P<ENC>ENC\([A-Za-z0-9+=/]*\))")?;
    let text = enc_r
        .replace_all(text, |captures: &regex::Captures| {
            let data = captures.get(1).map_or("", |m| m.as_str()).to_string();
            let data = &data[4..data.len() - 1];
            let data = hex::decode(data).expect("[Tardis.Config] Decryption error: invalid hex");
            crate::TardisFuns::crypto
                .aead
                .decrypt_ecb::<Aes128>(data, salt)
                .map(String::from_utf8)
                .expect("[Tardis.Config] Decryption error")
                .expect("[Tardis.Config] Uft8 decode error")
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
