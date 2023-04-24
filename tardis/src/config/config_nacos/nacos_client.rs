use std::{collections::HashMap, fmt::Display, io::Read, sync::Arc};

use derive_more::Display;
// use futures::TryFutureExt;
use crypto::{digest::Digest, md5::Md5};
use reqwest::{Error as ReqwestError, StatusCode};
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
    /// listener poll period, default 5s
    pub poll_period: std::time::Duration,
    access_token: Option<String>,
    reqwest_client: reqwest::Client,
}

impl Display for NacosClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NacosClient {{ base_url: {}, poll_period: {:?}, access_token: {:?} }}",
            self.base_url, self.poll_period, self.access_token,
        )
    }
}

impl Default for NacosClient {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8848/nacos".to_owned(),
            poll_period: std::time::Duration::from_secs(5),
            access_token: None,
            reqwest_client: reqwest::Client::new(),
        }
    }
}

impl NacosClient {
    /// create a new nacos client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// take access token as reqwest acceptable query
    fn access_token_as_query(&self) -> Vec<(&str, &str)> {
        self.access_token.as_ref().map(|token| (ACCESS_TOKEN_FIELD, token.as_str())).into_iter().collect()
    }

    /// authenticate with username and password
    pub async fn login(&mut self, username: &str, password: &str) -> Result<&mut Self, reqwest::Error> {
        let url = format!("{}/v1/auth/login", self.base_url);
        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);
        let access_token = self.reqwest_client.post(&url).form(&params).send().await?.json::<NacosAuthResponse>().await?.access_token;
        self.access_token = Some(access_token);
        Ok(self)
    }

    /// publish config by a nacos config descriptor and some content implement `Read`
    pub async fn publish_config(&mut self, descriptor: &NacosConfigDescriptor<'_>, content: &mut impl Read) -> Result<bool, NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}/v1/cs/configs", self.base_url);
        let mut params = HashMap::new();
        let mut content_buf = String::new();
        content.read_to_string(&mut content_buf).map_err(IoError)?;
        params.insert("content", content_buf);
        let resp = self.reqwest_client.post(&url).query(descriptor).query(&self.access_token_as_query()).form(&params).send().await;
        log::debug!("[Tardis.Config] publish_config resp: {:?}", resp);
        resp.map_err(ReqwestError)?.json::<bool>().await.map_err(ReqwestError)
    }

    /// get config by a nacos config descriptor
    pub async fn get_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<(StatusCode, String), NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}/v1/cs/configs", self.base_url);
        let resp = self.reqwest_client.get(&url).query(descriptor).query(&self.access_token_as_query()).send().await;
        match resp {
            Ok(resp) => {
                let status = resp.status();
                // only update md5 when status is success
                if status.is_success() {
                    let text = resp.text().await.map_err(ReqwestError)?;
                    descriptor.update_md5(&text).await;
                    Ok((status, text))
                } else {
                    Err(ReqwestError(resp.error_for_status().unwrap_err()))
                }
            }
            Err(e) => Err(ReqwestError(e)),
        }
    }

    /// delete config by a nacos config descriptor
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

    /// listen config change, if updated, return Ok(true), if not updated, return Ok(false)
    pub async fn listen_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<bool, NacosClientError> {
        use NacosClientError::*;
        {
            let md5 = descriptor.md5.lock().await;
            if md5.is_none() {
                return Ok(false);
            }
        }
        let url = format!("{}/v1/cs/configs/listener", self.base_url);
        let mut params = HashMap::new();
        params.insert("Listening-Configs", descriptor.as_listening_configs().await);
        log::debug!("[Tardis.Config] listen_config Listening-Configs: {:?}", params.get("Listening-Configs"));
        let resp = self
            .reqwest_client
            .post(&url)
            .header("Long-Pulling-Timeout", self.poll_period.as_millis().to_string())
            .query(&self.access_token_as_query())
            .query(&params)
            .send()
            .await
            .map_err(ReqwestError)?;
        log::debug!("[Tardis.Config] listen_config resp: {:?}", resp);
        let result = resp.text().await.map_err(ReqwestError)?;
        let result = if result.is_empty() { None } else { Some(result) };
        if let Some(config_text) = &result {
            {
                log::info!("[Tardis.Config] Listening-Configs {} updated", config_text);
                Ok(true)
            }
        } else {
            // not updated
            Ok(false)
        }
    }
}

/// # Nacos config descriptor
/// it's a descriptor corresponding to a config in nacos, it stores content's md5 value
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NacosConfigDescriptor<'a> {
    pub data_id: &'a str,
    pub group: &'a str,
    pub tenant: Option<&'a str>,
    #[serde(skip)]
    pub md5: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl<'a> NacosConfigDescriptor<'a> {
    /// create a new config descriptor
    pub fn new(data_id: &'a str, group: &'a str, md5: &Arc<tokio::sync::Mutex<Option<String>>>) -> Self {
        Self {
            data_id,
            group,
            tenant: None,
            md5: md5.clone(),
        }
    }

    /// update md5 value by content
    pub async fn update_md5(&self, content: &str) {
        let mut encoder = Md5::new();
        encoder.input_str(content);
        let result = encoder.result_str();
        self.md5.lock().await.replace(result);
    }

    /// data format: `dataId%02Group%02contentMD5%02tenant%01` or `dataId%02Group%02contentMD5%01`
    /// 
    /// md5 value could be empty string
    /// 
    /// refer: https://nacos.io/zh-cn/docs/open-api.html
    pub async fn as_listening_configs(&self) -> String {
        let spliter = 0x02 as char;
        let terminator = 0x01 as char;
        // if md5 is none then it will be empty string
        let mut result = {
            let md5_guard = self.md5.lock().await;
            let md5 = md5_guard.as_ref().map(String::as_str).unwrap_or("");
            let mut buf = vec![self.data_id, self.group, md5];
            buf.extend(self.tenant.iter());
            buf.join(&spliter.to_string())
        };
        result.push(terminator);
        result
    }
}
#[derive(Deserialize)]
struct NacosAuthResponse {
    #[serde(rename(deserialize = "accessToken"))]
    pub access_token: String,
}
