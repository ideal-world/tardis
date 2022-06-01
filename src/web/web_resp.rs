use crate::basic::error::TardisError;
use crate::basic::result::TARDIS_RESULT_SUCCESS_CODE;
use crate::serde::{Deserialize, Serialize};
use poem::http::StatusCode;
use poem_openapi::payload::Json;
use poem_openapi::{
    types::{ParseFromJSON, ToJSON},
    Object,
};

const TARDIS_ERROR_FLAG: &str = "__TARDIS_ERROR__";

pub type TardisApiResult<T> = poem::Result<Json<TardisResp<T>>>;

impl From<TardisError> for poem::Error {
    fn from(error: TardisError) -> Self {
        let status_code = match error {
            // TODO
            TardisError::Custom(_, _) => StatusCode::BAD_REQUEST,
            // TODO
            TardisError::_Inner(_) => StatusCode::BAD_REQUEST,
            // TODO
            TardisError::Box(_) => StatusCode::BAD_REQUEST,
            TardisError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TardisError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            TardisError::IOError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TardisError::BadRequest(_) => StatusCode::BAD_REQUEST,
            TardisError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            TardisError::NotFound(_) => StatusCode::NOT_FOUND,
            TardisError::FormatError(_) => StatusCode::BAD_REQUEST,
            TardisError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            TardisError::Conflict(_) => StatusCode::CONFLICT,
        };
        poem::Error::from_string(format!("{}{}", TARDIS_ERROR_FLAG, error), status_code)
    }
}

impl From<poem::Error> for TardisError {
    fn from(error: poem::Error) -> Self {
        let msg = error.to_string();
        if msg.starts_with(TARDIS_ERROR_FLAG) {
            let msg = msg.split_at(TARDIS_ERROR_FLAG.len()).1.to_string();
            return TardisError::form(&msg);
        }
        let msg = &error.to_string();
        let http_code = error.into_response().status();
        mapping_http_code_to_error(http_code, msg).unwrap()
    }
}

pub fn mapping_http_code_to_error(http_code: StatusCode, msg: &str) -> Option<TardisError> {
    if msg.starts_with(TARDIS_ERROR_FLAG) {
        let msg = msg.split_at(TARDIS_ERROR_FLAG.len()).1.to_string();
        return Some(TardisError::form(&msg));
    }
    match http_code {
        StatusCode::OK => None,
        StatusCode::BAD_REQUEST => Some(TardisError::BadRequest(msg.to_string())),
        StatusCode::UNAUTHORIZED => Some(TardisError::Unauthorized(msg.to_string())),
        StatusCode::NOT_FOUND => Some(TardisError::NotFound(msg.to_string())),
        StatusCode::INTERNAL_SERVER_ERROR => Some(TardisError::InternalError(msg.to_string())),
        StatusCode::SERVICE_UNAVAILABLE => Some(TardisError::InternalError(msg.to_string())),
        _ => Some(TardisError::BadRequest(msg.to_string())),
    }
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
pub struct TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub code: String,
    pub msg: String,
    pub data: Option<T>,
}

impl<T> TardisResp<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub fn ok(data: T) -> TardisApiResult<T> {
        TardisApiResult::Ok(Json(TardisResp {
            code: TARDIS_RESULT_SUCCESS_CODE.to_string(),
            msg: "".to_string(),
            data: Some(data),
        }))
    }

    pub fn err(error: TardisError) -> TardisApiResult<T> {
        TardisApiResult::Err(error.into())
    }
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
pub struct TardisPage<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub page_size: u64,
    pub page_number: u64,
    pub total_size: u64,
    pub records: Vec<T>,
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
pub struct Void {}
