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
    #[display(fmt = "000000000000##{:?}", _0)]
    Box(Box<dyn Error + Send + Sync>),
    #[display(fmt = "500000000000##{}", _0)]
    InternalError(String),
    #[display(fmt = "501000000000##{}", _0)]
    NotImplemented(String),
    #[display(fmt = "503000000000##{}", _0)]
    IOError(String),
    #[display(fmt = "400000000000##{}", _0)]
    BadRequest(String),
    #[display(fmt = "401000000000##{}", _0)]
    Unauthorized(String),
    #[display(fmt = "404000000000##{}", _0)]
    NotFound(String),
    #[display(fmt = "406000000000##{}", _0)]
    FormatError(String),
    #[display(fmt = "408000000000##{}", _0)]
    Timeout(String),
    #[display(fmt = "409000000000##{}", _0)]
    Conflict(String),
    #[display(fmt = "{}", _0)]
    _Inner(String),
}

impl TardisError {
    pub fn parse(msg: String) -> (String, String) {
        let split_idx = msg.find(GENERAL_SPLIT).expect("Illegal error description format");
        let code = &msg[..split_idx];
        let message = &msg[split_idx + 2..];
        (code.to_string(), message.to_string())
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
