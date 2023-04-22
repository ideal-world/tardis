use std::{collections::HashMap, fmt::Write, io::Read};

use derive_more::Display;
// use futures::TryFutureExt;
use reqwest::Error as ReqwestError;
use serde::{Deserialize, Serialize};

const ACCESS_TOKEN_FIELD: &str = "accessToken";

#[derive(Debug, Display)]
pub enum NacosClientError {
    ReqwestError(ReqwestError),
    IoError(std::io::Error),
    UrlParseError(url::ParseError),
}

impl std::error::Error for NacosClientError {}

/// for request nacos openapi, see https://nacos.io/zh-cn/docs/open-api.html
#[derive(Debug, Clone)]
pub struct NacosClient {
    base_url: String,
    poll_period: std::time::Duration,
    access_token: Option<String>,
    reqwest_client: reqwest::Client,
}

impl Default for NacosClient {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8848/nacos/".to_owned(),
            poll_period: std::time::Duration::from_secs(30),
            access_token: None,
            reqwest_client: reqwest::Client::new(),
        }
    }
}

impl NacosClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    pub fn access_token_as_query<'a>(&'a self) -> Vec<(&'a str, &'a str)> {
        self.access_token.as_ref().map(|token| (ACCESS_TOKEN_FIELD, token.as_str())).into_iter().collect()
    }
    pub async fn login(&mut self, username: &str, password: &str) -> Result<&mut Self, reqwest::Error> {
        let url = format!("{}/v1/auth/login", self.base_url);
        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);
        let access_token = self.reqwest_client.post(&url).form(&params).send().await?.json::<NacosAuthResponse>().await?.access_token;
        self.access_token = Some(access_token);
        Ok(self)
    }

    pub async fn publish_config(&mut self, descriptor: &NacosConfigDescriptor<'_>, content: &mut impl Read) -> Result<bool, NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}v1/cs/configs", self.base_url);
        let mut params = HashMap::new();
        let mut content_buf = String::new();
        content.read_to_string(&mut content_buf).map_err(IoError)?;
        params.insert("content", content_buf);
        self.reqwest_client
            .post(&url)
            .query(descriptor)
            .query(&self.access_token_as_query())
            .form(&params)
            .send()
            .await
            .map_err(ReqwestError)?
            .json::<bool>()
            .await
            .map_err(ReqwestError)
    }

    pub async fn get_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<String, NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}/v1/cs/configs", self.base_url);
        self.reqwest_client.post(&url).query(descriptor).query(&self.access_token_as_query()).send().await.map_err(ReqwestError)?.text().await.map_err(ReqwestError)
    }

    pub async fn delete_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<bool, NacosClientError> {
        use NacosClientError::*;
        let auth_url = format!("{}/v1/cs/configs", self.base_url);
        reqwest::Client::new()
            .delete(&auth_url)
            .query(descriptor)
            .query(&self.access_token_as_query())
            .send()
            .await
            .map_err(ReqwestError)?
            .json::<bool>()
            .await
            .map_err(ReqwestError)
    }

    pub async fn listen_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<Option<String>, NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}/v1/cs/configs/listener", self.base_url);
        let mut params = HashMap::new();
        params.insert("Listening-Configs", descriptor.as_listening_configs());
        self.reqwest_client
            .get(&url)
            .header("Long-Pulling-Timeout", self.poll_period.as_millis().to_string())
            .query(&self.access_token_as_query())
            .query(&params)
            .send()
            .await
            .map_err(ReqwestError)?
            .text()
            .await
            .map_err(ReqwestError)
            .map(|s| if s.is_empty() { None } else { Some(s) })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NacosConfigDescriptor<'a> {
    pub data_id: &'a str,
    pub group: &'a str,
    pub tenant: Option<&'a str>,
    // #[serde(rename(serialize = "type"))]
    // pub tp: Option<String>,
}

impl<'a> NacosConfigDescriptor<'a> {
    pub fn new(data_id: &'a str, group: &'a str) -> Self {
        Self {
            data_id,
            group,
            tenant: None,
        }
    }

    /// data format: dataId%02Group%02contentMD5%02tenant%01 or dataId%02Group%02contentMD5%01
    /// refer: https://nacos.io/zh-cn/docs/open-api.html
    pub fn as_listening_configs(&self) -> String {
        let mut buf = String::new();
        write!(buf, "{}%02{}%02", self.data_id, self.group).unwrap();
        if let Some(tenant) = self.tenant {
            write!(buf, "{}%02", tenant).unwrap();
        }
        write!(buf, "%01").unwrap();
        buf
    }
}
#[derive(Deserialize)]
struct NacosAuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
