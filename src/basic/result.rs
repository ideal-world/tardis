use core::result::Result;
use std::fmt::Display;

use derive_more::Display;

use crate::basic::error::TardisError;
use crate::basic::field::GENERAL_SPLIT;

pub type TardisResult<T> = Result<T, TardisError>;

#[derive(Display, Debug)]
pub enum StatusCodeKind {
    #[display(fmt = "200")]
    Success,
    #[display(fmt = "000")]
    UnKnown,
    #[display(fmt = "400")]
    BadRequest,
    #[display(fmt = "401")]
    Unauthorized,
    #[display(fmt = "404")]
    NotFound,
    #[display(fmt = "406")]
    FormatError,
    #[display(fmt = "408")]
    Timeout,
    #[display(fmt = "409")]
    Conflict,
    #[display(fmt = "419")]
    ConflictExists,
    #[display(fmt = "429")]
    ConflictExistFieldsAtSomeTime,
    #[display(fmt = "439")]
    ConflictExistAssociatedData,
    #[display(fmt = "500")]
    InternalError,
    #[display(fmt = "501")]
    NotImplemented,
    #[display(fmt = "503")]
    IOError,
}

impl StatusCodeKind {
    pub fn into_unified_code(&self) -> String {
        format!("{}000000000", self)
    }
}

#[derive(Display, Debug)]
pub enum ActionKind {
    #[display(fmt = "01")]
    Create,
    #[display(fmt = "02")]
    Modify,
    #[display(fmt = "03")]
    FetchOne,
    #[display(fmt = "04")]
    FetchList,
    #[display(fmt = "05")]
    Delete,
    #[display(fmt = "06")]
    Exists,
}

pub fn parse<E: Display>(content: E) -> (String, String) {
    let text = content.to_string();
    let split_idx = text.find(GENERAL_SPLIT).expect("Illegal error description format");
    let code = &text[..split_idx];
    let message = &text[split_idx + 2..];
    (code.to_string(), message.to_string())
}
