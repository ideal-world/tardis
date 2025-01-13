use crate::basic::error::TardisError;
use crate::basic::result::{TARDIS_RESULT_ACCEPTED_CODE, TARDIS_RESULT_SUCCESS_CODE};
use crate::serde::{Deserialize, Serialize};
use crate::TardisFuns;
use poem::http::StatusCode;
use poem::Response;
use poem_openapi::payload::Json;
use poem_openapi::{
    types::{ParseFromJSON, ToJSON},
    Object,
};

const TARDIS_ERROR_FLAG: &str = "__TARDIS_ERROR__";
pub const HEADER_X_TARDIS_ERROR: &str = "x-tardis-error";
pub type TardisApiResult<T> = poem::Result<Json<TardisResp<T>>>;

impl From<TardisError> for poem::Error {
    fn from(error: TardisError) -> Self {
        // If there's a better way, we may parse the status code from error.code[0..3] while its length is enough.
        let status_code = match &error.code {
            c if c.starts_with("400") => StatusCode::BAD_REQUEST,
            c if c.starts_with("401") => StatusCode::UNAUTHORIZED,
            c if c.starts_with("403") => StatusCode::FORBIDDEN,
            c if c.starts_with("404") => StatusCode::NOT_FOUND,
            c if c.starts_with("405") => StatusCode::METHOD_NOT_ALLOWED,
            c if c.starts_with("406") => StatusCode::NOT_ACCEPTABLE,
            c if c.starts_with("408") => StatusCode::REQUEST_TIMEOUT,
            c if c.starts_with("409") => StatusCode::CONFLICT,
            c if c.starts_with("410") => StatusCode::GONE,
            c if c.starts_with("411") => StatusCode::LENGTH_REQUIRED,
            c if c.starts_with("412") => StatusCode::PRECONDITION_FAILED,
            c if c.starts_with("413") => StatusCode::PAYLOAD_TOO_LARGE,
            c if c.starts_with("414") => StatusCode::URI_TOO_LONG,
            c if c.starts_with("415") => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            c if c.starts_with("416") => StatusCode::RANGE_NOT_SATISFIABLE,
            c if c.starts_with("417") => StatusCode::EXPECTATION_FAILED,
            c if c.starts_with("418") => StatusCode::IM_A_TEAPOT,
            c if c.starts_with("421") => StatusCode::MISDIRECTED_REQUEST,
            c if c.starts_with("422") => StatusCode::UNPROCESSABLE_ENTITY,
            c if c.starts_with("423") => StatusCode::LOCKED,
            c if c.starts_with("424") => StatusCode::FAILED_DEPENDENCY,
            c if c.starts_with("426") => StatusCode::UPGRADE_REQUIRED,
            c if c.starts_with("428") => StatusCode::PRECONDITION_REQUIRED,
            c if c.starts_with("429") => StatusCode::TOO_MANY_REQUESTS,
            c if c.starts_with("431") => StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            c if c.starts_with("451") => StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            c if c.starts_with("500") => StatusCode::INTERNAL_SERVER_ERROR,
            c if c.starts_with("501") => StatusCode::NOT_IMPLEMENTED,
            c if c.starts_with("502") => StatusCode::BAD_GATEWAY,
            c if c.starts_with("503") => StatusCode::SERVICE_UNAVAILABLE,
            c if c.starts_with("504") => StatusCode::GATEWAY_TIMEOUT,
            c if c.starts_with("505") => StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            c if c.starts_with("506") => StatusCode::VARIANT_ALSO_NEGOTIATES,
            c if c.starts_with("507") => StatusCode::INSUFFICIENT_STORAGE,
            c if c.starts_with("508") => StatusCode::LOOP_DETECTED,
            c if c.starts_with("510") => StatusCode::NOT_EXTENDED,
            c if c.starts_with("511") => StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let response = Response::builder().header(HEADER_X_TARDIS_ERROR, &error.code).status(status_code).body(format!(
            "{}{}",
            TARDIS_ERROR_FLAG,
            TardisFuns::json.obj_to_string(&error).unwrap_or_else(|_| String::new())
        ));
        poem::Error::from_response(response)
    }
}

pub fn mapping_http_code_to_error(http_code: StatusCode, msg: &str) -> Option<TardisError> {
    if let Some(tardis_error) = msg.strip_prefix(TARDIS_ERROR_FLAG) {
        let error = TardisFuns::json.str_to_obj(tardis_error).unwrap_or_else(|_| TardisError::format_error("[Tardis.WebServer] Invalid format error", "406-tardis-error-invalid"));
        return Some(error);
    }
    match http_code {
        code if code.as_u16() < 400 => None,
        _ => Some(TardisError::custom(
            http_code.as_str(),
            &format!("[Tardis.WebServer] Process error: {msg}"),
            &format!(
                "{}-tardis-webserver-error",
                if [400, 401, 404, 406, 408, 409, 500, 501].contains(&http_code.as_u16()) {
                    http_code.as_str()
                } else {
                    "-1"
                }
            ),
        )),
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
            msg: String::new(),
            data: Some(data),
        }))
    }

    pub fn accepted(data: T) -> TardisApiResult<T> {
        TardisApiResult::Ok(Json(TardisResp {
            code: TARDIS_RESULT_ACCEPTED_CODE.to_string(),
            msg: String::new(),
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

#[derive(Object, Serialize, Clone, Debug, Default, Copy)]
/// This `Void` is for represent an empty value.
/// Any value can be deserialized as `Void`.
/// Void will be serialized as json's `null`.
pub struct Void;
impl<'de> Deserialize<'de> for Void {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // ignore this value whatever
        let _ = deserializer.deserialize_any(serde::de::IgnoredAny)?;
        Ok(Void)
    }
}
