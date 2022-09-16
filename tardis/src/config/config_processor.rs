use async_trait::async_trait;
use config::builder::AsyncState;
use config::{AsyncSource, ConfigBuilder, ConfigError, Environment, File, FileFormat, Format, Map};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::basic::error::TardisError;
use crate::basic::fetch_profile;
use crate::basic::locale::TardisLocale;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::config::config_nacos::ConfNacosProcessor;
use crate::log::{debug, info};
use std::fmt::Debug;

use super::config_dto::{ConfCenterConfig, TardisConfig};

/// Configuration handle / 配置处理
///
/// Organizing Configuration Management with Tardis Best Practices
///
/// 使用 Tardis 最佳实践组织配置管理
impl TardisConfig {
    pub(crate) async fn init(relative_path: &str) -> TardisResult<TardisConfig> {
        let profile = fetch_profile();
        let path = Path::new(relative_path);
        let parent_path = env::current_dir().expect("[Tardis.Config] Current path get error");

        info!(
            "[Tardis.Config] Initializing, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );

        let mut config = TardisConfig::do_init(relative_path, &profile, None).await?;

        #[cfg(feature = "web-client")]
        {
            config = if let Some(conf_center) = &config.fw.conf_center {
                let format = match conf_center.format.as_ref().unwrap_or(&"toml".to_string()).to_lowercase().as_str() {
                    "toml" => FileFormat::Toml,
                    "json" => FileFormat::Json,
                    "yaml" => FileFormat::Yaml,
                    _ => {
                        return Err(TardisError::format_error(
                            "[Tardis.Config] The file format of configcenter only supports [toml,json,yaml]",
                            "",
                        ))
                    }
                };
                let conf_center_urls = match conf_center.kind.to_lowercase().as_str() {
                    "nacos" => ConfNacosProcessor::fetch_conf_urls(&profile, &config.fw.app.id, conf_center).await?,
                    _ => return Err(TardisError::format_error("[Tardis.Config] The kind of configcenter only supports [nacos]", "")),
                };
                TardisConfig::do_init(relative_path, &profile, Some((conf_center_urls, format))).await?
            } else {
                config
            };
        }

        info!(
            "[Tardis.Config] Initialized, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        debug!("=====[Tardis.Config] Content=====\n{:#?}\n=====", &config.fw);

        TardisLocale::init(path)?;
        Ok(config)
    }

    async fn do_init(relative_path: &str, profile: &str, conf_center: Option<(Vec<String>, FileFormat)>) -> TardisResult<TardisConfig> {
        let mut conf = ConfigBuilder::<AsyncState>::default();

        let path = Path::new(relative_path);

        // Fetch from local file
        if !relative_path.is_empty() {
            conf = conf.add_source(File::from(path.join("conf-default")).required(true));
            if !profile.is_empty() {
                conf = conf.add_source(File::from(path.join(format!("conf-{}", profile).as_str())).required(true));
            }
        }

        #[cfg(feature = "web-client")]
        {
            // Fetch from remote
            if let Some(conf_center) = conf_center {
                for conf_center_url in conf_center.0 {
                    conf = conf.add_async_source(HttpSource {
                        url: conf_center_url,
                        format: conf_center.1,
                    });
                }
            }
        }

        // Fetch from ENV
        conf = conf.add_source(Environment::with_prefix("TARDIS"));
        let conf = conf.build().await?;

        let mut workspace_config: HashMap<String, Value> = Default::default();
        match conf.get::<Value>("cs") {
            Ok(c) => {
                workspace_config.insert("".to_string(), c);
            }
            Err(e) => match e {
                ConfigError::NotFound(_) => {
                    info!("[Tardis.Config] No [cs] configuration found");
                }
                _ => return Err(e.into()),
            },
        }
        match conf.get::<HashMap<String, Value>>("csm") {
            Ok(c) => {
                workspace_config.extend(c);
            }
            Err(e) => match e {
                ConfigError::NotFound(_) => {
                    info!("[Tardis.Config] No [csm] configuration found");
                }
                _ => return Err(e.into()),
            },
        }
        let framework_config = conf.get::<FrameworkConfig>("fw")?;

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

#[derive(Debug)]
pub(crate) struct HttpSource<F: Format> {
    url: String,
    format: F,
}

#[async_trait]
pub(crate) trait ConfCenterProcess {
    async fn fetch_conf_urls(profile: &str, app_id: &str, config: &ConfCenterConfig) -> TardisResult<Vec<String>>;
}

#[async_trait]
impl<F> AsyncSource for HttpSource<F>
where
    F: Format + Send + Sync + Debug,
{
    async fn collect(&self) -> Result<Map<String, config::Value>, ConfigError> {
        reqwest::get(&self.url)
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))?
            .text()
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))
            .and_then(|text| self.format.parse(Some(&self.url), &text).map_err(|e| ConfigError::Foreign(e)))
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
            ConfigError::Frozen => TardisError::io_error(&format!("[Tardis.Config] {:?}", error), "503-tardis-config-frozen"),
            ConfigError::NotFound(_) => TardisError::not_found(&format!("[Tardis.Config] {:?}", error), "404-tardis-config-not-exist"),
            ConfigError::PathParse(_) => TardisError::format_error(&format!("[Tardis.Config] {:?}", error), "406-tardis-config-parse-error"),
            ConfigError::FileParse { .. } => TardisError::format_error(&format!("[Tardis.Config] {:?}", error), "406-tardis-config-parse-error"),
            ConfigError::Type { .. } => TardisError::format_error(&format!("[Tardis.Config] {:?}", error), "406-tardis-config-parse-error"),
            ConfigError::Message(s) => TardisError::wrap(&format!("[Tardis.Config] {:?}", s), "-1-tardis-config-custom-error"),
            ConfigError::Foreign(err) => TardisError::wrap(&format!("[Tardis.Config] {:?}", err), "-1-tardis-config-error"),
        }
    }
}
