use std::collections::HashMap;

use async_trait::async_trait;
use config::ConfigError;
use serde::Deserialize;

use crate::basic::result::TardisResult;

use super::{config_dto::ConfCenterConfig, config_processor::ConfCenterProcess};

pub(crate) struct ConfNacosProcessor<'a> {
    pub(crate) config: &'a ConfCenterConfig,
    pub(crate) access_token: Option<String>,
}

impl<'a> ConfNacosProcessor<'a> {
    pub fn new(config: &'a ConfCenterConfig) -> Self {
        Self {
            config,
            access_token: None,
        }
    }
    pub async fn fetch_access_token(&mut self) -> TardisResult<()> {
        let config = &self.config;
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
        self.access_token = Some(access_token);
        Ok(())
    }
    pub async fn access_token(&mut self) -> TardisResult<String> {
        if self.access_token.is_none() {
            self.fetch_access_token().await?;
        }
        Ok(self.access_token.clone().unwrap())
    }
}

#[async_trait]
impl<'a> ConfCenterProcess for ConfNacosProcessor<'a> {
    async fn fetch_conf_urls(&mut self, profile: &str, app_id: &str) -> TardisResult<Vec<String>> {
        let access_token = self.access_token().await?;
        let config = &self.config;
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

    async fn fetch_conf_listener_urls(&mut self, profile: &str, app_id: &str, content_md5: Option<&str>) -> TardisResult<Vec<String>> {
        let access_token = self.access_token().await?;
        let config = &self.config;
        let tenant = if let Some(namespace) = &config.namespace {
            format!("&tenant={namespace}")
        } else {
            "".to_string()
        };
        let group = config.group.as_ref().unwrap_or(&"DEFAULT_GROUP".to_string()).to_string();


        let content_md5 = content_md5.unwrap_or("");
        let mut listener_urls = vec![format!(
            "{}/v1/cs/configs/listener?accessToken={access_token}&Listening-Configs={app_id}-default%02{group}%02{content_md5}%02{tenant}%01",
            config.url
        )];
        if !profile.is_empty() {
            listener_urls.push(format!(
                "{}/v1/cs/configs/listener?accessToken={access_token}&Listening-Configs={app_id}-{profile}%02{group}%02{content_md5}%02{tenant}%01",
                config.url
            ));
        }
        Ok(listener_urls)
        
    }
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
