use std::convert::Infallible;
use std::error::Error;
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use derive_more::Display;

use crate::basic::field::GENERAL_SPLIT;

pub static ERROR_DEFAULT_CODE: &str = "-1";

/// Tardis unified error wrapper / Tardis统一错误封装
#[derive(Display, Debug)]
pub enum TardisError {
    #[display(fmt = "{}##{}", _0, _1)]
    Custom(String, String),
    #[display(fmt = "000##{:?}", _0)]
    Box(Box<dyn Error + Send + Sync>),
    #[display(fmt = "500##{}", _0)]
    InternalError(String),
    #[display(fmt = "501##{}", _0)]
    NotImplemented(String),
    #[display(fmt = "503##{}", _0)]
    IOError(String),
    #[display(fmt = "400##{}", _0)]
    BadRequest(String),
    #[display(fmt = "401##{}", _0)]
    Unauthorized(String),
    #[display(fmt = "404##{}", _0)]
    NotFound(String),
    #[display(fmt = "406##{}", _0)]
    FormatError(String),
    #[display(fmt = "408##{}", _0)]
    Timeout(String),
    #[display(fmt = "409##{}", _0)]
    Conflict(String),
    #[display(fmt = "000##{}", _0)]
    _Inner(String),
}

impl TardisError {
    pub fn form(msg: &str) -> TardisError {
        let (code, message) = Self::to_tuple(msg.to_string());
        TardisError::Custom(code, message)
    }

    fn to_tuple(msg: String) -> (String, String) {
        let split_idx = msg.find(GENERAL_SPLIT).expect("Illegal error description format");
        let code = &msg[..split_idx];
        let message = &msg[split_idx + 2..];
        (code.to_string(), message.to_string())
    }

    pub fn parse(&self) -> (String, String) {
        Self::to_tuple(self.to_string())
    }

    pub fn new(code: u16, msg: &str) -> Option<Self> {
        match code {
            c if (200..300).contains(&c) => None,
            500 => Some(Self::InternalError(msg.to_string())),
            501 => Some(Self::NotImplemented(msg.to_string())),
            503 => Some(Self::IOError(msg.to_string())),
            400 => Some(Self::BadRequest(msg.to_string())),
            401 => Some(Self::Unauthorized(msg.to_string())),
            404 => Some(Self::NotFound(msg.to_string())),
            406 => Some(Self::FormatError(msg.to_string())),
            408 => Some(Self::Timeout(msg.to_string())),
            409 => Some(Self::Conflict(msg.to_string())),
            _ => Some(Self::Custom(code.to_string(), msg.to_string())),
        }
    }

    pub fn code(&self) -> String {
        let text = self.to_string();
        let split_idx = text.find(GENERAL_SPLIT).expect("Illegal error description format");
        let code = &text[..split_idx];
        code.to_string()
    }

    pub fn message(&self) -> String {
        let text = self.to_string();
        let split_idx = text.find(GENERAL_SPLIT).expect("Illegal error description format");
        let message = &text[split_idx + 2..];
        message.to_string()
    }
}

pub struct TardisErrorWithExt {
    pub ext: String,
}

impl TardisErrorWithExt {
    pub fn internal_error(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("500-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn not_implemented(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("501-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn io_error(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("503-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn bad_request(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("400-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn unauthorized(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("401-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn not_found(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("404-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn format_error(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("406-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn timeout(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("408-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }

    pub fn conflict(&self, obj_name: &str, obj_opt: &str, msg: &str) -> TardisError {
        TardisError::Custom(format!("409-{}-{}-{}", self.ext, obj_name, obj_opt), msg.to_string())
    }
}

impl From<std::io::Error> for TardisError {
    fn from(error: std::io::Error) -> Self {
        TardisError::IOError(error.to_string())
    }
}

impl From<Utf8Error> for TardisError {
    fn from(error: Utf8Error) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<FromUtf8Error> for TardisError {
    fn from(error: FromUtf8Error) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<url::ParseError> for TardisError {
    fn from(error: url::ParseError) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<ParseIntError> for TardisError {
    fn from(error: ParseIntError) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<Infallible> for TardisError {
    fn from(error: Infallible) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<base64::DecodeError> for TardisError {
    fn from(error: base64::DecodeError) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<hex::FromHexError> for TardisError {
    fn from(error: hex::FromHexError) -> Self {
        TardisError::FormatError(error.to_string())
    }
}

impl From<regex::Error> for TardisError {
    fn from(error: regex::Error) -> Self {
        TardisError::FormatError(error.to_string())
    }
}
