use std::collections::HashMap;

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
        let mut params = HashMap::new();
        params.insert("username", &config.username);
        params.insert("password", &config.password);
        let access_token = reqwest::Client::new()
            .post(&auth_url)
            .form(&params)
            .send()
            .await
            .map_err(|error| ConfigError::Foreign(Box::new(error)))?
            .json::<AuthResponse>()
            .await
            .map_err(|error| ConfigError::Foreign(Box::new(error)))?
            .access_token;
        let tenant = if let Some(namespace) = &config.namespace {
            format!("&tenant={namespace}")
        } else {
            "".to_string()
        };
        let group = config.group.as_ref().unwrap_or(&"DEFAULT_GROUP".to_string()).to_string();

        let mut config_urls = vec![format!(
            "{}/v1/cs/configs?accessToken={}&dataId={}-default&group={}{}",
            config.url, access_token, app_id, group, tenant
        )];
        if !profile.is_empty() {
            config_urls.push(format!(
                "{}/v1/cs/configs?accessToken={}&dataId={}-{}&group={}{}",
                config.url, access_token, app_id, profile, group, tenant
            ));
        }
        Ok(config_urls)
    }
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
