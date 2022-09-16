use config::{Config, ConfigError, Environment, File};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::basic::error::TardisError;
use crate::basic::fetch_profile;
use crate::basic::locale::TardisLocale;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::log::{debug, info};

use super::config_dto::TardisConfig;

/// Configuration handle / 配置处理
///
/// Organizing Configuration Management with Tardis Best Practices
///
/// 使用 Tardis 最佳实践组织配置管理
impl TardisConfig {
    pub(crate) fn init(relative_path: &str) -> TardisResult<TardisConfig> {
        let profile = fetch_profile();
        let path = Path::new(relative_path);

        let parent_path = env::current_dir().expect("[Tardis.Config] Current path get error");

        info!(
            "[Tardis.Config] Initializing, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        let mut conf = Config::builder();
        if !relative_path.is_empty() {
            conf = conf.add_source(File::from(path.join("conf-default")).required(true));
            if !profile.is_empty() {
                conf = conf.add_source(File::from(path.join(format!("conf-{}", profile).as_str())).required(true));
            }
        }
        conf = conf.add_source(Environment::with_prefix("TARDIS"));
        let conf = conf.build()?;
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

        info!(
            "[Tardis.Config] Initialized, base path:{:?}, relative path:{:?}, profile:{}",
            parent_path, relative_path, profile
        );
        debug!("=====[Tardis.Config] Content=====\n{:#?}\n=====", framework_config);

        let config = if framework_config.adv.salt.is_empty() {
            Ok(TardisConfig {
                cs: workspace_config,
                fw: framework_config,
            })
        } else {
            #[cfg(not(feature = "crypto"))]
            return Err(TardisError::format_error("[Tardis.Config] Configuration encryption must depend on the crypto feature", ""));
            #[cfg(feature = "crypto")]
            {
                // decryption processing
                let salt = framework_config.adv.salt.clone();
                if salt.len() != 16 {
                    return Err(TardisError::format_error("[Tardis.Config] [salt] Length must be 16", ""));
                }
                fn decryption(text: &str, salt: &str) -> String {
                    let enc_r = regex::Regex::new(r"(?P<ENC>ENC\([A-Za-z0-9+/]*\))").unwrap();
                    enc_r
                        .replace_all(text, |captures: &regex::Captures| {
                            let data = captures.get(1).map_or("", |m| m.as_str()).to_string();
                            let data = &data[4..data.len() - 1];
                            crate::TardisFuns::crypto.aes.decrypt_ecb(data, salt).expect("[Tardis.Config] Decryption error")
                        })
                        .to_string()
                }
                let wc = decryption(&crate::TardisFuns::json.obj_to_string(&workspace_config)?, &salt);
                let fw = decryption(&crate::TardisFuns::json.obj_to_string(&framework_config)?, &salt);
                let workspace_config = crate::TardisFuns::json.str_to_obj(&wc)?;
                let framework_config = crate::TardisFuns::json.str_to_obj(&fw)?;
                Ok(TardisConfig {
                    cs: workspace_config,
                    fw: framework_config,
                })
            }
        };

        TardisLocale::init(path)?;

        config
    }
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
