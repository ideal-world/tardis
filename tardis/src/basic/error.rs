use crate::basic::locale::TardisLocale;
use crate::serde::{Deserialize, Serialize};
use core::fmt::Display;
use std::convert::Infallible;
use std::num::{ParseIntError, TryFromIntError};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTimeError;
use tracing::warn;

pub static ERROR_DEFAULT_CODE: &str = "-1";

/// Tardis unified error wrapper / Tardis统一错误封装
#[derive(Deserialize, Serialize, Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct TardisError {
    pub code: String,
    pub message: String,
}

impl Display for TardisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.code, self.message)
    }
}

impl TardisError {
    fn error(code: &str, msg: &str, locale_code: &str) -> TardisError {
        warn!("[Tardis.Error] {}:{}", code, msg);
        let message = TardisLocale::env_message(if locale_code.trim().is_empty() { code } else { locale_code }, msg);
        TardisError { code: code.to_string(), message }
    }

    #[must_use]
    pub fn internal_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("500", msg, locale_code)
    }

    #[must_use]
    pub fn not_implemented(msg: &str, locale_code: &str) -> TardisError {
        Self::error("501", msg, locale_code)
    }
    #[must_use]
    pub fn bad_gateway(msg: &str, locale_code: &str) -> TardisError {
        Self::error("502", msg, locale_code)
    }
    #[must_use]
    pub fn io_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("503", msg, locale_code)
    }
    #[must_use]
    pub fn gateway_timeout(msg: &str, locale_code: &str) -> TardisError {
        Self::error("504", msg, locale_code)
    }
    #[must_use]
    pub fn bad_request(msg: &str, locale_code: &str) -> TardisError {
        Self::error("400", msg, locale_code)
    }
    #[must_use]
    pub fn unauthorized(msg: &str, locale_code: &str) -> TardisError {
        Self::error("401", msg, locale_code)
    }
    #[must_use]
    pub fn forbidden(msg: &str, locale_code: &str) -> TardisError {
        Self::error("403", msg, locale_code)
    }
    #[must_use]
    pub fn not_found(msg: &str, locale_code: &str) -> TardisError {
        Self::error("404", msg, locale_code)
    }
    #[must_use]
    pub fn format_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("406", msg, locale_code)
    }
    #[must_use]
    pub fn timeout(msg: &str, locale_code: &str) -> TardisError {
        Self::error("408", msg, locale_code)
    }
    #[must_use]
    pub fn conflict(msg: &str, locale_code: &str) -> TardisError {
        Self::error("409", msg, locale_code)
    }
    #[must_use]
    pub fn custom(code: &str, msg: &str, locale_code: &str) -> TardisError {
        Self::error(code, msg, locale_code)
    }
    #[must_use]
    pub fn wrap(msg: &str, locale_code: &str) -> TardisError {
        Self::error(ERROR_DEFAULT_CODE, msg, locale_code)
    }
}

impl std::error::Error for TardisError {}

pub struct TardisErrorWithExt {
    pub ext: String,
    /// <https://www.andiamo.co.uk/resources/iso-language-codes/>
    pub lang: Option<String>,
}

impl TardisErrorWithExt {
    #[must_use]
    pub fn error(&self, code: &str, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        let code = format!("{}-{}-{}-{}", code, self.ext, obj_name, obj_opt);
        warn!("[Tardis.Error] {}:{}", code, msg);
        let message = self.localized_message(if locale_code.trim().is_empty() { &code } else { locale_code }, msg);
        TardisError { code, message }
    }
    #[must_use]
    pub fn localized_message(&self, locale_code: &str, msg: &str) -> String {
        if let Some(lang) = &self.lang {
            TardisLocale::get_message(locale_code, msg, lang).unwrap_or_else(|_| msg.to_string())
        } else {
            msg.to_string()
        }
    }
    #[must_use]
    pub fn internal_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("500", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn not_implemented(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("501", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn io_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("503", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn bad_request(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("400", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn unauthorized(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("401", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn not_found(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("404", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn format_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("406", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn timeout(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("408", obj_name, obj_opt, msg, locale_code)
    }
    #[must_use]
    pub fn conflict(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("409", obj_name, obj_opt, msg, locale_code)
    }
}

// dynamic cast any error into TardisError
impl From<&dyn std::error::Error> for TardisError {
    fn from(error: &dyn std::error::Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<std::io::Error> for TardisError {
    fn from(error: std::io::Error) -> Self {
        TardisError::io_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<Utf8Error> for TardisError {
    fn from(error: Utf8Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<FromUtf8Error> for TardisError {
    fn from(error: FromUtf8Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<url::ParseError> for TardisError {
    fn from(error: url::ParseError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<ParseIntError> for TardisError {
    fn from(error: ParseIntError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<Infallible> for TardisError {
    fn from(error: Infallible) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<base64::DecodeError> for TardisError {
    fn from(error: base64::DecodeError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<hex::FromHexError> for TardisError {
    fn from(error: hex::FromHexError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<regex::Error> for TardisError {
    fn from(error: regex::Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<TryFromIntError> for TardisError {
    fn from(error: TryFromIntError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl<P> From<PoisonError<RwLockReadGuard<'_, P>>> for TardisError {
    fn from(error: PoisonError<RwLockReadGuard<'_, P>>) -> Self {
        TardisError::conflict(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl<P> From<PoisonError<RwLockWriteGuard<'_, P>>> for TardisError {
    fn from(error: PoisonError<RwLockWriteGuard<'_, P>>) -> Self {
        TardisError::conflict(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<tracing_subscriber::reload::Error> for TardisError {
    fn from(error: tracing_subscriber::reload::Error) -> Self {
        TardisError::internal_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

impl From<SystemTimeError> for TardisError {
    fn from(error: SystemTimeError) -> Self {
        TardisError::internal_error(&format!("[Tardis.Basic] {error}"), "")
    }
}

#[cfg(feature = "tracing")]
impl From<opentelemetry_sdk::trace::TraceError> for TardisError {
    fn from(error: opentelemetry_sdk::trace::TraceError) -> Self {
        TardisError::internal_error(&format!("[Tardis.Basic] {error}"), "")
    }
}
