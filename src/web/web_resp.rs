use async_trait::async_trait;
use poem::error::{CorsError, MethodNotAllowedError, NotFoundError, ParsePathError};
use poem::http::StatusCode;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response};
use poem_openapi::error::{AuthorizationError, ContentTypeError, ParseJsonError, ParseMultipartError, ParseParamError};
use poem_openapi::payload::Payload;
use poem_openapi::registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry};
use poem_openapi::{
    types::{ParseFromJSON, ToJSON},
    ApiResponse, Object,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{trace, warn};

use crate::basic::error::TardisError;
use crate::basic::result::{parse, StatusCodeKind};
use crate::TardisFuns;

#[derive(Deserialize, Serialize, Debug)]
#[serde(default)]
pub struct TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub code: String,
    pub msg: String,
    pub data: Option<T>,
}

impl<T> Default for TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    fn default() -> Self {
        TardisResp {
            code: "".to_string(),
            msg: "".to_string(),
            data: None,
        }
    }
}

impl<T> TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub fn ok(data: T) -> Self {
        Self {
            code: StatusCodeKind::Success.to_string(),
            msg: "".to_string(),
            data: Some(data),
        }
    }

    pub fn err(error: TardisError) -> Self {
        let (code, msg) = parse(error);
        let msg = process_err_msg(code.as_str(), msg);
        Self { code, msg, data: None }
    }
}

impl<T> IntoResponse for TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json; charset=utf8")
            .body(TardisFuns::json.obj_to_string(&self).expect("[Tardis.WebClient] Response body parsing error"))
    }
}

impl<T> Payload for TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    const CONTENT_TYPE: &'static str = "application/json";
    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

impl<T> ApiResponse for TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: Self::CONTENT_TYPE,
                    schema: Self::schema_ref(),
                }],
                headers: vec![],
            }],
        }
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
pub struct TardisPage<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub page_size: usize,
    pub page_number: usize,
    pub total_size: usize,
    pub records: Vec<T>,
}

pub struct UniformError;

impl<E: Endpoint> Middleware<E> for UniformError {
    type Output = UniformErrorImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        UniformErrorImpl(ep)
    }
}

pub struct UniformErrorImpl<E>(E);

#[async_trait]
impl<E: Endpoint> Endpoint for UniformErrorImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let method = req.method().to_string();
        let url = req.uri().to_string();
        let resp = self.0.call(req).await;
        match resp {
            Ok(resp) => {
                let mut resp = resp.into_response();
                if resp.status() == StatusCode::OK {
                    return Ok(resp);
                }
                let msg = resp.take_body().into_string().await.expect("[Tardis.WebClient] Request exception type conversion error");
                let code = if resp.status().as_u16() >= 500 {
                    warn!(
                        "[Tardis.WebServer] Process error,request method:{}, url:{}, response\
                             code:{}, message:{}",
                        method,
                        url,
                        resp.status().as_u16(),
                        msg
                    );
                    resp.status()
                } else {
                    trace!(
                        "[Tardis.WebServer] Process error,request method:{}, url:{}, response code:{}, message:{}",
                        method,
                        url,
                        resp.status().as_u16(),
                        msg
                    );
                    // Request fallback friendly
                    StatusCode::OK
                };
                resp.set_status(code);
                resp.headers_mut().insert(
                    "Content-Type",
                    "application/json; charset=utf8".parse().expect("[Tardis.WebServer] Http head parsing error"),
                );
                let code = mapping_code(code).into_unified_code();
                resp.set_body(
                    json!({
                        "code": code,
                        "msg": process_err_msg(code.as_str(),msg),
                    })
                    .to_string(),
                );
                Ok(resp)
            }
            Err(err) => Ok(error_handler(err)),
        }
    }
}

fn mapping_code(http_code: StatusCode) -> StatusCodeKind {
    match http_code {
        StatusCode::OK => StatusCodeKind::Success,
        StatusCode::BAD_REQUEST => StatusCodeKind::BadRequest,
        StatusCode::UNAUTHORIZED => StatusCodeKind::Unauthorized,
        StatusCode::FORBIDDEN => StatusCodeKind::NotFound,
        StatusCode::NOT_FOUND => StatusCodeKind::NotFound,
        StatusCode::METHOD_NOT_ALLOWED => StatusCodeKind::NotFound,
        StatusCode::INTERNAL_SERVER_ERROR => StatusCodeKind::InternalError,
        StatusCode::SERVICE_UNAVAILABLE => StatusCodeKind::InternalError,
        _ => StatusCodeKind::UnKnown,
    }
}

fn error_handler(err: poem::Error) -> Response {
    let (code, msg) =
        if err.is::<ParseParamError>() || err.is::<ParseJsonError>() || err.is::<ParseMultipartError>() || err.is::<ParsePathError>() || err.is::<MethodNotAllowedError>() {
            (StatusCodeKind::BadRequest.into_unified_code(), err.to_string())
        } else if err.is::<NotFoundError>() || err.is::<ContentTypeError>() {
            (StatusCodeKind::NotFound.into_unified_code(), err.to_string())
        } else if err.is::<AuthorizationError>() || err.is::<CorsError>() {
            (StatusCodeKind::Unauthorized.into_unified_code(), err.to_string())
        } else {
            warn!("[Tardis.WebServer] Process error: {:?}", err);
            (StatusCodeKind::UnKnown.into_unified_code(), err.to_string())
        };
    Response::builder().status(StatusCode::OK).header("Content-Type", "application/json; charset=utf8").body(
        json!({
            "code": code,
            "msg": process_err_msg(code.as_str(),msg),
        })
        .to_string(),
    )
}

fn process_err_msg(code: &str, msg: String) -> String {
    if TardisFuns::fw_config().web_server.security_hide_err_msg {
        warn!("[Tardis.WebServer] Pesponse error,code:{},msg:{}", code, msg);
        "Security is enabled, detailed errors are hidden, please check the server logs".to_string()
    } else {
        msg
    }
}
