use crate::basic::error::TardisError;
use core::result::Result;

/// Tardis return object wrapper / Tardis返回对象封装
pub type TardisResult<T> = Result<T, TardisError>;

pub const TARDIS_RESULT_SUCCESS_CODE: &str = "200";
