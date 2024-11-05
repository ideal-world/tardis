use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Client, IntoUrl, Method, RequestBuilder, Response};
use serde::Deserialize;
use tokio::io::AsyncRead;
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
pub trait TardisRequestBody {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder;
}

impl TardisRequestBody for () {
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder {
        builder
    }
}

/// Async Read for [`TardisWebClient`],
pub struct Read<R>(R);

impl<R> TardisRequestBody for Read<R>
where
    R: AsyncRead + Send + Sync + Unpin + 'static,
{
    fn apply_on(self, builder: RequestBuilder) -> RequestBuilder {
        let stream = tokio_util::io::ReaderStream::new(self.0);
        builder.body(reqwest::Body::wrap_stream(stream))
    }
}

/// Plain text body for [`TardisWebClient`],
pub struct PlainText<T>(pub T);

/// Json body for [`TardisWebClient`],
pub struct Json<'a, T>(pub &'a T);

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

/// convert a str pair into a string pair, it may be helpful when you want to use a string literal as a header for [`TardisWebClient`]
pub fn str_pair_to_string_pair(p: (&str, &str)) -> (String, String) {
    (p.0.to_owned(), p.1.to_owned())
}

pub trait DebugUrl: IntoUrl + std::fmt::Debug {}
impl<T> DebugUrl for T where T: IntoUrl + std::fmt::Debug {}
impl TardisWebClient {
    /// # Errors
    /// Return error if the client cannot be created.
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

    /// Get and parse response body as text
    pub async fn get_to_str(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::GET, url, headers, ()).await?;
        self.to_text(code, headers, response).await
    }

    /// Get and parse response body as json
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::GET, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Head and ignore response body
    pub async fn head_to_void(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request(Method::HEAD, url, headers, ()).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    /// Head and parse response body as json
    pub async fn head<T: for<'de> Deserialize<'de>>(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::HEAD, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Delete and ignore response body
    pub async fn delete_to_void(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<()>> {
        let (code, headers, _) = self.request(Method::DELETE, url, headers, ()).await?;
        Ok(TardisHttpResponse { code, headers, body: None })
    }

    /// Delete and parse response body as json
    pub async fn delete<T: for<'de> Deserialize<'de>>(&self, url: impl DebugUrl, headers: impl IntoIterator<Item = (String, String)>) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::DELETE, url, headers, ()).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Delete and parse response body as json with a body
    pub async fn delete_with_body<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: impl DebugUrl,
        headers: impl IntoIterator<Item = (String, String)>,
        body: &B,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::DELETE, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Post and ignore response body
    pub async fn post_str_to_str(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Post and parse response body as json
    pub async fn post_obj_to_str<B: Serialize>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Post and parse response body as json
    pub async fn post_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Post and parse response body as json
    pub async fn post<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::POST, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Put and ignore response body
    pub async fn put_str_to_str(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Put and parse response body as json
    pub async fn put_obj_to_str<B: Serialize>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Put and parse response body as json
    pub async fn put_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Put and parse response body as json
    pub async fn put<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PUT, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Patch and ignore response body
    pub async fn patch_str_to_str(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, PlainText(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Patch and parse response body as json
    pub async fn patch_obj_to_str<B: Serialize>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<String>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, Json(body)).await?;
        self.to_text(code, headers, response).await
    }

    /// Patch and parse response body as json
    pub async fn patch_to_obj<T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: impl Into<String>,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, PlainText(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }

    /// Patch and parse response body as json
    pub async fn patch<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: impl DebugUrl,
        body: &B,
        headers: impl IntoIterator<Item = (String, String)>,
    ) -> TardisResult<TardisHttpResponse<T>> {
        let (code, headers, response) = self.request(Method::PATCH, url, headers, Json(body)).await?;
        self.to_json::<T>(code, headers, response).await
    }
    #[tracing::instrument(name="send_http_request", skip_all, fields(method=?method, url=?url))]
    pub async fn request<K, V>(
        &self,
        method: Method,
        url: impl DebugUrl,
        headers: impl IntoIterator<Item = (K, V)>,
        body: impl TardisRequestBody,
    ) -> TardisResult<(u16, HashMap<String, String>, Response)>
    where
        K: Into<String>,
        V: Into<String>,
    {
        let mut url = url.into_url()?;
        TardisFuns::uri.sort_url_query(&mut url);
        let mut result = self.client.request(method, url.clone());
        for (key, value) in &self.default_headers {
            result = result.header(key, value);
        }
        for (key, value) in headers {
            result = result.header(key.into(), value.into());
        }
        #[allow(unused_mut)]
        let mut request = body.apply_on(result).build()?;
        #[cfg(feature = "tracing")]
        {
            use opentelemetry::{global, Context};
            let ctx = Context::current();
            global::get_text_map_propagator(|propagator| propagator.inject_context(&ctx, &mut crate::basic::tracing::HeaderInjector(request.headers_mut())));
        }
        trace!("start request");
        let response = self.client.execute(request).await?;
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
        trace!(code, "response received");
        Ok((code, headers, response))
    }

    pub async fn to_text(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<String>> {
        match response.text().await {
            Ok(body) => Ok(TardisHttpResponse { code, headers, body: Some(body) }),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.WebClient] {error:?}"), "406-tardis-webclient-text-error")),
        }
    }

    pub async fn to_json<T: for<'de> Deserialize<'de>>(&self, code: u16, headers: HashMap<String, String>, response: Response) -> TardisResult<TardisHttpResponse<T>> {
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
