use async_trait::async_trait;
use poem::http::StatusCode;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response};
use tracing::{trace, warn};

use crate::basic::error::TardisError;
use crate::basic::result::TARDIS_RESULT_SUCCESS_CODE;
use crate::serde_json::json;
use crate::TardisFuns;

use super::web_resp::mapping_http_code_to_error;

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
                        "[Tardis.WebServer] Process error,request method:{}, url:{}, response code:{}, message:{}",
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

                let (bus_code, msg) = if let Some(error) = mapping_http_code_to_error(http_code, &msg) {
                    error.parse()
                } else {
                    (TARDIS_RESULT_SUCCESS_CODE.to_string(), "".to_string())
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
            Err(err) => {
                let error: TardisError = err.into();
                let (bus_code, msg) = error.parse();
                Ok(Response::builder().status(StatusCode::OK).header("Content-Type", "application/json; charset=utf8").body(
                    json!({
                        "code": bus_code,
                        "msg": process_err_msg(bus_code.as_str(),msg),
                    })
                    .to_string(),
                ))
            }
        }
    }
}

fn process_err_msg(code: &str, msg: String) -> String {
    if TardisFuns::fw_config().web_server.security_hide_err_msg {
        warn!("[Tardis.WebServer] Pesponse error,code:{},msg:{}", code, msg);
        "Security is enabled, detailed errors are hidden, please check the server logs".to_string()
    } else {
        msg
    }
}
