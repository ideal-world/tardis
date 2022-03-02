use poem::error::{CorsError, MethodNotAllowedError, NotFoundError, ParsePathError};
use poem::http::StatusCode;
use poem_openapi::error::{AuthorizationError, ContentTypeError, ParseMultipartError, ParseParamError, ParseRequestPayloadError};
use poem_openapi::payload::Json;
use poem_openapi::{
    types::{ParseFromJSON, ToJSON},
    Object,
};
use tracing::warn;

use crate::basic::error::TardisError;
use crate::basic::result::StatusCodeKind;
use crate::serde::{Deserialize, Serialize};

pub const TARDIS_ERROR_FLAG: &str = "__TARDIS_ERROR__";

pub type TardisApiResult<T> = poem::Result<Json<TardisResp<T>>>;

impl From<TardisError> for poem::Error {
    fn from(error: TardisError) -> Self {
        let status_code = match error {
            // TODO
            TardisError::Custom(_, _) => StatusCode::BAD_REQUEST,
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
            // TODO
            TardisError::_Inner(_) => StatusCode::BAD_REQUEST,
        };
        poem::Error::from_string(format!("{}{}", TARDIS_ERROR_FLAG, error), status_code)
    }
}

impl From<poem::Error> for TardisError {
    fn from(error: poem::Error) -> Self {
        if error.is::<ParseParamError>()
            || error.is::<ParseRequestPayloadError>()
            || error.is::<ParseMultipartError>()
            || error.is::<ContentTypeError>()
            || error.is::<ParsePathError>()
            || error.is::<MethodNotAllowedError>()
        {
            TardisError::BadRequest(error.to_string())
        } else if error.is::<NotFoundError>() {
            TardisError::NotFound(error.to_string())
        } else if error.is::<AuthorizationError>() || error.is::<CorsError>() {
            TardisError::Unauthorized(error.to_string())
        } else {
            warn!("[Tardis.WebServer] Process error kind: {:?}", error);
            TardisError::_Inner(error.to_string())
        }
    }
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
#[oai(inline)]
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
            code: StatusCodeKind::Success.into_unified_code(),
            msg: "".to_string(),
            data: Some(data),
        }))
    }

    pub fn err(error: TardisError) -> TardisApiResult<T> {
        TardisApiResult::Err(error.into())
    }
}

#[derive(Object, Deserialize, Serialize, Clone, Debug)]
#[oai(inline)]
pub struct TardisPage<T>
where
    T: ParseFromJSON + ToJSON + Serialize + Send + Sync,
{
    pub page_size: u64,
    pub page_number: u64,
    pub total_size: u64,
    pub records: Vec<T>,
}
