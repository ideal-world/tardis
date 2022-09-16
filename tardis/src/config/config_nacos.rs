use async_trait::async_trait;
use config::ConfigError;
use serde::Deserialize;

use crate::basic::result::TardisResult;

use super::{config_dto::ConfCenterConfig, config_processor::ConfCenterProcess};

pub(crate) struct ConfNacosProcessor;

#[async_trait]
impl ConfCenterProcess for ConfNacosProcessor {
    async fn fetch_conf_urls(profile: &str, app_id: &str, config: &ConfCenterConfig) -> TardisResult<Vec<String>> {
        let auth_url = format!("{}/v1/auth/login", config.url);
        let access_token = reqwest::Client::new()
            .post(&auth_url)
            .body(format!("username={}&password={}", config.username, config.password))
            .send()
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))?
            .json::<AuthResponse>()
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))?
            .access_token;
        let tenant = if let Some(namespace) = &config.namespace {
            format!("&tenant={}", namespace)
        } else {
            "".to_string()
        };
        let format = config.format.as_ref().unwrap_or(&"toml".to_string()).to_string();
        let group = config.group.as_ref().unwrap_or(&"default".to_string()).to_string();

        Ok(vec![
            format!(
                "{}/v1/cs/configs?accessToken={}&dataId={}-default.{}&group={}{}",
                config.url, access_token, app_id, format, group, tenant
            ),
            format!(
                "{}/v1/cs/configs?accessToken={}&dataId={}-{}.{}&group={}{}",
                config.url, access_token, app_id, profile, format, group, tenant
            ),
        ])
    }
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
