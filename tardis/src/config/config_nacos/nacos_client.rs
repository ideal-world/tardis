use std::{collections::HashMap, io::Read, sync::Arc};

use derive_more::Display;
// use futures::TryFutureExt;
use reqwest::Error as ReqwestError;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, trace};

use crate::TardisFuns;

const ACCESS_TOKEN_FIELD: &str = "accessToken";

#[derive(Debug, Display)]
pub enum NacosClientError {
    ReqwestError(ReqwestError),
    IoError(std::io::Error),
    UrlParseError(url::ParseError),
    NoAuth,
}

impl From<reqwest::Error> for NacosClientError {
    fn from(value: reqwest::Error) -> Self {
        NacosClientError::ReqwestError(value)
    }
}
impl std::error::Error for NacosClientError {}

/// for request nacos openapi, see <https://nacos.io/zh-cn/docs/open-api.html>
#[derive(Debug, Clone)]
pub struct NacosClient {
    pub base_url: String,
    /// listener poll period, default 5s
    pub poll_period: std::time::Duration,
    access_token: Arc<Mutex<Option<String>>>,
    pub reqwest_client: reqwest::Client,
    auth: Option<(String, String)>,
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
            access_token: Default::default(),
            reqwest_client: reqwest::Client::new(),
            auth: None,
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

    /// create a new nacos client
    pub fn new_with_client(base_url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into(),
            reqwest_client: client,
            ..Default::default()
        }
    }

    /// get a copy of current using access_token
    pub async fn get_current_access_token(&self) -> Option<String> {
        self.access_token.lock().await.clone()
    }

    /// execute a reqwest::Request, with access_token if client has one
    pub async fn reqwest_execute(&self, f: impl Fn(&reqwest::Client) -> reqwest::RequestBuilder) -> Result<reqwest::Response, reqwest::Error> {
        let mut request = f(&self.reqwest_client);
        if let Some(access_token) = self.access_token.lock().await.as_deref() {
            request = request.query(&[(ACCESS_TOKEN_FIELD, access_token)]);
        };
        self.reqwest_client.execute(request.build()?).await
    }

    #[cfg(feature = "test")]
    /// create a new nacos client, with danger option
    /// # Safety
    /// **⚠ Don't use this in production environment ⚠**
    /// # Panic
    /// panic when reqwest_client build failed
    pub fn new_test(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            reqwest_client: reqwest::Client::builder().danger_accept_invalid_certs(true).build().expect("fail to build test NacosClient"),
            ..Default::default()
        }
    }

    /// take access token as reqwest acceptable query
    async fn access_token_as_query(&self) -> Vec<(String, String)> {
        let access_token = self.access_token.lock().await;
        access_token.as_ref().map(|token| (ACCESS_TOKEN_FIELD.into(), token.clone())).into_iter().collect()
    }

    /// authenticate with username and password
    pub async fn login(&mut self, username: &str, password: &str) -> Result<&mut Self, reqwest::Error> {
        let url = format!("{}/v1/auth/login", self.base_url);
        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);
        let access_token = self.reqwest_client.post(&url).form(&params).send().await?.json::<NacosAuthResponse>().await?.access_token;
        {
            *(self.access_token.lock().await) = Some(access_token);
        }
        self.auth = Some((username.to_string(), password.to_string()));
        Ok(self)
    }

    pub async fn relogin(&self) -> Result<&Self, NacosClientError> {
        if let Some((ref username, ref password)) = self.auth.clone() {
            debug!("[Tardis.Config] Trying to re-login");
            // lock while re-login
            let mut access_token_lock = self.access_token.lock().await;
            let url = format!("{}/v1/auth/login", self.base_url);
            let mut params = HashMap::new();
            params.insert("username", username);
            params.insert("password", password);
            let access_token = self.reqwest_client.post(&url).form(&params).send().await?.json::<NacosAuthResponse>().await?.access_token;
            *access_token_lock = Some(access_token);
            // release lock
            drop(access_token_lock);
            debug!("[Tardis.Config] Success to re-login");
            Ok(self)
        } else {
            Err(NacosClientError::NoAuth)
        }
    }

    /// publish config by a nacos config descriptor and some content implement `Read`
    pub async fn publish_config(&mut self, descriptor: &NacosConfigDescriptor<'_>, content: &mut impl Read) -> Result<bool, NacosClientError> {
        use NacosClientError::*;
        let url = format!("{}/v1/cs/configs", self.base_url);
        let mut params = HashMap::new();
        let mut content_buf = String::new();
        content.read_to_string(&mut content_buf).map_err(IoError)?;
        params.insert("content", content_buf);
        let mut resp = self.reqwest_client.post(&url).query(descriptor).query(&self.access_token_as_query().await).form(&params).send().await?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            self.relogin().await?;
            resp = self.reqwest_client.post(&url).query(descriptor).query(&self.access_token_as_query().await).form(&params).send().await?;
        }
        Ok(resp.json::<bool>().await?)
    }

    /// get config by a nacos config descriptor
    pub async fn get_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<String, NacosClientError> {
        let url = format!("{}/v1/cs/configs", self.base_url);
        let mut resp = self.reqwest_client.get(&url).query(descriptor).query(&self.access_token_as_query().await).send().await?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            resp = self.reqwest_client.get(&url).query(descriptor).query(&self.access_token_as_query().await).send().await?;
        }
        resp = resp.error_for_status()?;
        let text = resp.text().await?;
        descriptor.update_by_content(&text).await;
        Ok(text)
    }

    /// delete config by a nacos config descriptor
    pub async fn delete_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<bool, NacosClientError> {
        let auth_url = format!("{}/v1/cs/configs", self.base_url);
        let mut resp = self.reqwest_client.delete(&auth_url).query(descriptor).query(&self.access_token_as_query().await).send().await?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            resp = self.reqwest_client.delete(&auth_url).query(descriptor).query(&self.access_token_as_query().await).send().await?
        }

        Ok(resp.json::<bool>().await?)
    }

    async fn listen_config_inner<T: Serialize>(&self, params: &T) -> Result<reqwest::Response, NacosClientError> {
        let resp = self
            .reqwest_client
            .post(format!("{}/v1/cs/configs/listener", self.base_url))
            // refer: https://nacos.io/zh-cn/docs/open-api.html
            // doc says it's `pulling` instead of `polling`
            .header("Long-Pulling-Timeout", self.poll_period.as_millis().to_string())
            .query(&self.access_token_as_query().await)
            .query(&params)
            .send()
            .await?;
        Ok(resp)
    }

    /// listen config change, if updated, return Ok(true), if not updated, return Ok(false)
    pub async fn listen_config(&self, descriptor: &NacosConfigDescriptor<'_>) -> Result<bool, NacosClientError> {
        {
            let md5 = descriptor.md5.lock().await;
            if md5.is_none() {
                return Ok(false);
            }
        }
        let mut params = HashMap::new();
        params.insert("Listening-Configs", descriptor.as_listening_configs().await);
        trace!("[Tardis.Config] listen_config Listening-Configs: {:?}", params.get("Listening-Configs"));

        let mut resp = self.listen_config_inner(&params).await?;

        trace!("[Tardis.Config] listen_config resp: {:?}", resp);
        // case of token expired
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            self.relogin().await?;
            resp = self.listen_config_inner(&params).await?;
        };
        let result = resp.text().await?;
        let result = if result.is_empty() { None } else { Some(result) };
        if let Some(config_text) = &result {
            {
                debug!("[Tardis.Config] update with digest {}", config_text);
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
    pub async fn update_by_content(&self, content: &str) {
        let md5 = TardisFuns::crypto.digest.md5(content).expect("fail to calculate md5");
        self.md5.lock().await.replace(md5);
    }

    /// data format: `dataId%02Group%02contentMD5%02tenant%01` or `dataId%02Group%02contentMD5%01`
    ///
    /// md5 value could be empty string
    ///
    /// refer: <https://nacos.io/zh-cn/docs/open-api.html>
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
