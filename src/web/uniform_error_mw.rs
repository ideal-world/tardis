use async_trait::async_trait;
use poem::error::{CorsError, MethodNotAllowedError, NotFoundError, ParsePathError};
use poem::http::StatusCode;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response};
use poem_openapi::error::{AuthorizationError, ContentTypeError, ParseMultipartError, ParseParamError, ParseRequestPayloadError};
use tracing::{trace, warn};

use crate::basic::error::TardisError;
use crate::basic::result::StatusCodeKind;
use crate::serde_json::json;
use crate::{web, TardisFuns};

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

                let http_code = if resp.status().as_u16() >= 500 {
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
                resp.set_status(http_code);
                resp.headers_mut().insert(
                    "Content-Type",
                    "application/json; charset=utf8".parse().expect("[Tardis.WebServer] Http head parsing error"),
                );

                let (bus_code, msg) = if msg.starts_with(web::web_resp::TARDIS_ERROR_FLAG) {
                    let msg = msg.split_at(web::web_resp::TARDIS_ERROR_FLAG.len()).1.to_string();
                    TardisError::parse(msg)
                } else {
                    (mapping_code(http_code).into_unified_code(), msg)
                };
                resp.set_body(
                    json!({
                        "code": bus_code,
                        "msg": process_err_msg(bus_code.as_str(),msg),
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
    let msg = err.to_string();
    let (bus_code, msg) = if msg.starts_with(web::web_resp::TARDIS_ERROR_FLAG) {
        let msg = msg.split_at(web::web_resp::TARDIS_ERROR_FLAG.len()).1.to_string();
        TardisError::parse(msg)
    } else if err.is::<ParseParamError>()
        || err.is::<ParseRequestPayloadError>()
        || err.is::<ParseMultipartError>()
        || err.is::<ContentTypeError>()
        || err.is::<ParsePathError>()
        || err.is::<MethodNotAllowedError>()
    {
        (StatusCodeKind::BadRequest.into_unified_code(), err.to_string())
    } else if err.is::<NotFoundError>() {
        (StatusCodeKind::NotFound.into_unified_code(), err.to_string())
    } else if err.is::<AuthorizationError>() || err.is::<CorsError>() {
        (StatusCodeKind::Unauthorized.into_unified_code(), err.to_string())
    } else {
        warn!("[Tardis.WebServer] Process error kind: {:?}", err);
        (StatusCodeKind::UnKnown.into_unified_code(), err.to_string())
    };
    // TODO
    // let http_code = if bus_code.starts_with('5') {
    //
    //     StatusCode::INTERNAL_SERVER_ERROR
    // } else {
    //     StatusCode::OK
    // };
    Response::builder().status(StatusCode::OK).header("Content-Type", "application/json; charset=utf8").body(
        json!({
            "code": bus_code,
            "msg": process_err_msg(bus_code.as_str(),msg),
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
