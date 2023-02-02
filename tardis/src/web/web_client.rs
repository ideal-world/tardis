use std::collections::HashMap;
use std::time::Duration;

use log::{error, trace};
use reqwest::{Client, Method, Response};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::log::info;
use crate::serde::de::DeserializeOwned;
use crate::serde::Serialize;
use crate::TardisFuns;

pub struct TardisWebClient {
    default_headers: Vec<(String, String)>,
    client: Client,
}

impl TardisWebClient {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisWebClient>> {
        let mut clients = HashMap::new();
        clients.insert("".to_string(), TardisWebClient::init(conf.web_client.connect_timeout_sec)?);
        for (k, v) in &conf.web_client.modules {
            clients.insert(k.to_string(), TardisWebClient::init(v.connect_timeout_sec)?);
        }
        Ok(clients)
    }

    pub fn init(connect_timeout_sec: u64) -> TardisResult<TardisWebClient> {
        info!("[Tardis.WebClient] Initializing");
        let client = reqwest::Client::builder().danger_accept_invalid_certs(true).connect_timeout(Duration::from_secs(connect_timeout_sec)).https_only(false).build()?;
        info!("[Tardis.WebClient] Initialized");
        TardisResult::Ok(TardisWebClient {
            client,
            default_headers: Vec::new(),
        })
    }

    pub fn set_default_header(&mut self, key: &str, value: &str) {
        trace!("[Tardis.WebClient] Set default header: {}={}", key, value);
        self.default_headers.push((key.to_string(), value.to_string()));
    }

    pub fn remove_default_header(&mut self, key: &str) {
        trace!("[Tardis.WebClient] Remove default header: {}", key);
        self.default_headers.retain(|(k, _)| k != key);
    }

    pub async fn get_to_str(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<()>(Method::GET, url, headers, None, None).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn get<T: DeserializeOwned>(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::GET, url, headers, None, None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn head_to_void(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request::<()>(Method::HEAD, url, headers, None, None).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    pub async fn head<T: DeserializeOwned>(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::HEAD, url, headers, None, None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn delete_to_void(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request::<()>(Method::DELETE, url, headers, None, None).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    pub async fn delete<T: DeserializeOwned>(&self, url: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::DELETE, url, headers, None, None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn post_str_to_str(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<()>(Method::POST, url, headers, None, Some(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn post_obj_to_str<B: Serialize>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<B>(Method::POST, url, headers, Some(body), None).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn post_to_obj<T: DeserializeOwned>(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::POST, url, headers, None, Some(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, Some(body), None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn put_str_to_str(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<()>(Method::PUT, url, headers, None, Some(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn put_obj_to_str<B: Serialize>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<B>(Method::PUT, url, headers, Some(body), None).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn put_to_obj<T: DeserializeOwned>(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::PUT, url, headers, None, Some(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn put<B: Serialize, T: DeserializeOwned>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, Some(body), None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn patch_str_to_str(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<()>(Method::PATCH, url, headers, None, Some(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn patch_obj_to_str<B: Serialize>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request::<B>(Method::PATCH, url, headers, Some(body), None).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn patch_to_obj<T: DeserializeOwned>(&self, url: &str, body: &str, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request::<()>(Method::PATCH, url, headers, None, Some(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn patch<B: Serialize, T: DeserializeOwned>(&self, url: &str, body: &B, headers: Option<Vec<(String, String)>>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, Some(body), None).await?;
        self.to_json::<T>(code, headers, response).await
    }

    async fn request<B: Serialize>(
        &self,
        method: Method,
        url: &str,
        headers: Option<Vec<(String, String)>>,
        body: Option<&B>,
        str_body: Option<&str>,
    ) -> TardisResult<(u16, HashMap<String, String>, Response)> {
        let formatted_url = TardisFuns::uri.format(url)?;
        let method_str = method.to_string();
        trace!("[Tardis.WebClient] Request {}:{}", method_str, &formatted_url);
        let mut result = self.client.request(method, formatted_url.clone());
        for (key, value) in &self.default_headers {
            result = result.header(key, value);
        }
        if let Some(headers) = headers {
            for (key, value) in headers {
                result = result.header(key, value);
            }
        }
        if let Some(body) = body {
            result = result.json(body);
        }
        if let Some(body) = str_body {
            result = result.body(body.to_string());
        }
        let response = result.send().await?;
        let code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v.to_str().unwrap_or_else(|_| panic!("[Tardis.WebClient] Http head {v:?} parsing error")).to_string(),
                )
            })
            .collect();
        trace!("[Tardis.WebClient] Request {}:{}, Response {}", method_str, formatted_url, code);
        Ok((code, headers, response))
    }

    async fn to_text(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<String>> {
        match response.text().await {
            Ok(body) => Ok(TardisHttpResponse { code, headers, body: Some(body) }),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.WebClient] {error:?}"), "406-tardis-webclient-text-error")),
        }
    }

    async fn to_json<T: DeserializeOwned>(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<T>> {
        match response.json().await {
            Ok(body) => Ok(TardisHttpResponse { code, headers, body: Some(body) }),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.WebClient] {error:?}"), "406-tardis-webclient-json-error")),
        }
    }

    pub fn raw(&self) -> &Client {
        &self.client
    }
}

#[derive(Debug, Clone)]
pub struct TardisHttpResponse<T> {
    pub code: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<T>,
}

impl From<reqwest::Error> for TardisError {
    fn from(error: reqwest::Error) -> Self {
        error!("[Tardis.WebClient] Error: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.WebClient] {error:?}"), "-1-tardis-webclient-error")
    }
}
