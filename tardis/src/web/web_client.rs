use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Client, IntoUrl, Method, RequestBuilder, Response};
use serde::Deserialize;
use tracing::{error, info, trace};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::config_dto::component::web_client::WebClientModuleConfig;
use crate::serde::Serialize;
use crate::utils::initializer::InitBy;
use crate::TardisFuns;

pub struct TardisWebClient {
    default_headers: Vec<(String, String)>,
    client: Client,
}

#[async_trait::async_trait]
impl InitBy<WebClientModuleConfig> for TardisWebClient {
    async fn init_by(config: &WebClientModuleConfig) -> TardisResult<Self> {
        Self::init(config)
    }
}
trait TardisRequestBody {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder;
}

impl TardisRequestBody for () {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder {
        builder
    }
}
struct PlainText<T>(T);
struct Json<'a, T>(&'a T);

impl<T: Into<String>> TardisRequestBody for PlainText<T> {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder {
        builder.body(self.0.into())
    }
}

impl<T: Serialize> TardisRequestBody for Json<'_, T> {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder {
        builder.json(&self.0)
    }
}

impl TardisWebClient {
    pub fn init(WebClientModuleConfig { connect_timeout_sec, .. }: &WebClientModuleConfig) -> TardisResult<TardisWebClient> {
        info!("[Tardis.WebClient] Initializing");
        let client = reqwest::Client::builder().danger_accept_invalid_certs(true).connect_timeout(Duration::from_secs(*connect_timeout_sec)).https_only(false).build()?;
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

    pub async fn get_to_str(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::GET, url, headers, ()).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::GET, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn head_to_void(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request(Method::HEAD, url, headers, ()).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    pub async fn head<T: for<'de> Deserialize<'de>>(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::HEAD, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn delete_to_void(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request(Method::DELETE, url, headers, ()).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    pub async fn delete<T: for<'de> Deserialize<'de>>(&self, url: impl IntoUrl, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::DELETE, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn post_str_to_str(&self, url: impl IntoUrl, body: impl Into<String>, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn post_obj_to_str<B: Serialize>(&self, url: impl IntoUrl, body: &B, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn post_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn post<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: &B,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn put_str_to_str(&self, url: impl IntoUrl, body: impl Into<String>, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn put_obj_to_str<B: Serialize>(&self, url: impl IntoUrl, body: &B, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn put_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn put<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: &B,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn patch_str_to_str(&self, url: impl IntoUrl, body: impl Into<String>, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn patch_obj_to_str<B: Serialize>(&self, url: impl IntoUrl, body: &B, headers: impl IntoIterator<Item = (&str, &str)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    pub async fn patch_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    pub async fn patch<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl IntoUrl,
        body: &B,
        headers: impl IntoIterator<Item = (&str, &str)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    async fn request(
        &self,
        method: Method,
        url: impl IntoUrl,
        headers: impl IntoIterator<Item = (&str, &str)>,
        body: impl TardisRequestBody,
    ) -> TardisResult<(u16, HashMap<String, String>, Response)> {
        let mut url = url.into_url()?;
        TardisFuns::uri.sort_url_query(&mut url);
        let method_str = method.to_string();
        trace!("[Tardis.WebClient] Request {}:{}", method_str, &url);
        let mut result = self.client.request(method, url.clone());
        for (key, value) in &self.default_headers {
            result = result.header(key, value);
        }
        for (key, value) in headers {
            result = result.header(key, value);
        }
        result = body.apply_on(result);
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
        trace!("[Tardis.WebClient] Request {}:{}, Response {}", method_str, url, code);
        Ok((code, headers, response))
    }

    async fn to_text(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<String>> {
        match response.text().await {
            Ok(body) => Ok(TardisHttpResponse { code, headers, body: Some(body) }),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.WebClient] {error:?}"), "406-tardis-webclient-text-error")),
        }
    }

    async fn to_json<T: for<'de> Deserialize<'de>>(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<T>> {
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
