use core::fmt::Display;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::num::ParseIntError;
use std::path::Path;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Mutex;

use log::{info, warn};
use regex::Regex;

use crate::serde::{Deserialize, Serialize};
use crate::{TardisFuns, TardisResult};

pub static ERROR_DEFAULT_CODE: &str = "-1";

lazy_static! {
    static ref LOCALE_CONFIG: Mutex<HashMap<String, HashMap<String, (String, Option<Regex>)>>> = Mutex::new(HashMap::new());
}

/// Tardis unified error wrapper / Tardis统一错误封装
#[derive(Deserialize, Serialize, Clone, Debug)]
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
    pub fn init_locale(path: &Path) -> TardisResult<()> {
        let path = path.join("locale");
        if !path.exists() {
            return Ok(());
        }
        info!("[Tardis.Error] Initializing, base path:{:?}", path);
        let mut conf = LOCALE_CONFIG.lock().map_err(|e| TardisError::internal_error(&format!("{:?}", e), ""))?;
        let paths = path.read_dir().map_err(|e| TardisError::bad_request(&format!("[Tardis.Error] Path {:#?} dir error:{:#?}", path, e), ""))?;
        for entry in paths {
            let entry = entry?;
            let name = entry.file_name();
            let lang = name
                .to_str()
                .ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Error] File name error {:#?}", entry), ""))?
                // Ignore module name, just take language flag
                .split('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            if lang.is_empty() {
                continue;
            }
            let reader = BufReader::new(File::open(entry.path())?);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let mut items = line.split('\t');
                let code = items.next().ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Error] Code not exist in {}", line), ""))?.trim();
                let message = items.next().ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Error] Message not exist in {}", line), ""))?.trim();
                let regex = if let Some(regex) = items.next() {
                    Some(Regex::new(regex.trim()).map_err(|_| TardisError::bad_request(&format!("[Tardis.Error] Regex illegal in {}", line), ""))?)
                } else {
                    None
                };
                conf.entry(lang.to_string()).or_insert_with(HashMap::new).insert(code.to_string(), (message.to_string(), regex));
            }
        }
        Ok(())
    }

    pub fn get_localized_message(code: &str, default_message: &str, lang: &str) -> TardisResult<String> {
        let lang = lang.to_lowercase();
        let conf = LOCALE_CONFIG.lock().map_err(|e| TardisError::conflict(&format!("[Tardis.Error] locale config lock error: {:?}", e), ""))?;
        if let Some(conf) = conf.get(&lang) {
            if let Some((message, regex)) = conf.get(code) {
                let mut localized_message = message.clone();
                if let Some(regex) = regex {
                    return if let Some(cap) = regex.captures(default_message) {
                        for (idx, cap) in cap.iter().enumerate() {
                            if let Some(cap) = cap {
                                localized_message = localized_message.replace(&format!("{{{}}}", idx), cap.as_str());
                            }
                        }
                        Ok(localized_message)
                    } else {
                        // Regex not match, fallback to default message
                        Ok(default_message.to_string())
                    };
                }
                // No regex, return default message
                return Ok(message.to_string());
            }
        }
        // No locale config, return default message
        Ok(default_message.to_string())
    }

    fn error(code: &str, msg: &str, locale_code: &str) -> TardisError {
        warn!("[Tardis.Error] {}:{}", code, msg);
        let message = Self::localized_message(if locale_code.trim().is_empty() { code } else { locale_code }, msg);
        TardisError { code: code.to_string(), message }
    }

    pub fn localized_message(locale_code: &str, msg: &str) -> String {
        if let Some(lang) = &TardisFuns::default_lang() {
            TardisError::get_localized_message(locale_code, msg, lang).unwrap_or_else(|_| msg.to_string())
        } else {
            msg.to_string()
        }
    }

    pub fn internal_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("500", msg, locale_code)
    }

    pub fn not_implemented(msg: &str, locale_code: &str) -> TardisError {
        Self::error("501", msg, locale_code)
    }

    pub fn io_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("503", msg, locale_code)
    }

    pub fn bad_request(msg: &str, locale_code: &str) -> TardisError {
        Self::error("400", msg, locale_code)
    }

    pub fn unauthorized(msg: &str, locale_code: &str) -> TardisError {
        Self::error("401", msg, locale_code)
    }

    pub fn not_found(msg: &str, locale_code: &str) -> TardisError {
        Self::error("404", msg, locale_code)
    }

    pub fn format_error(msg: &str, locale_code: &str) -> TardisError {
        Self::error("406", msg, locale_code)
    }

    pub fn timeout(msg: &str, locale_code: &str) -> TardisError {
        Self::error("408", msg, locale_code)
    }

    pub fn conflict(msg: &str, locale_code: &str) -> TardisError {
        Self::error("409", msg, locale_code)
    }

    pub fn custom(code: &str, msg: &str, locale_code: &str) -> TardisError {
        Self::error(code, msg, locale_code)
    }

    pub fn wrap(msg: &str, locale_code: &str) -> TardisError {
        Self::error(ERROR_DEFAULT_CODE, msg, locale_code)
    }
}

pub struct TardisErrorWithExt {
    pub ext: String,
    /// https://www.andiamo.co.uk/resources/iso-language-codes/
    pub lang: Option<String>,
}

impl TardisErrorWithExt {
    fn error(&self, code: &str, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        let code = format!("{}-{}-{}-{}", code, self.ext, obj_name, obj_opt);
        warn!("[Tardis.Error] {}:{}", code, msg);
        let message = self.localized_message(if locale_code.trim().is_empty() { &code } else { locale_code }, msg);
        TardisError { code, message }
    }

    pub fn localized_message(&self, locale_code: &str, msg: &str) -> String {
        if let Some(lang) = &self.lang {
            TardisError::get_localized_message(locale_code, msg, lang).unwrap_or_else(|_| msg.to_string())
        } else {
            msg.to_string()
        }
    }

    pub fn internal_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("500", obj_name, obj_opt, msg, locale_code)
    }

    pub fn not_implemented(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("501", obj_name, obj_opt, msg, locale_code)
    }

    pub fn io_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("503", obj_name, obj_opt, msg, locale_code)
    }

    pub fn bad_request(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("400", obj_name, obj_opt, msg, locale_code)
    }

    pub fn unauthorized(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("401", obj_name, obj_opt, msg, locale_code)
    }

    pub fn not_found(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("404", obj_name, obj_opt, msg, locale_code)
    }

    pub fn format_error(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("406", obj_name, obj_opt, msg, locale_code)
    }

    pub fn timeout(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("408", obj_name, obj_opt, msg, locale_code)
    }

    pub fn conflict(&self, obj_name: &str, obj_opt: &str, msg: &str, locale_code: &str) -> TardisError {
        self.error("409", obj_name, obj_opt, msg, locale_code)
    }
}

impl From<std::io::Error> for TardisError {
    fn from(error: std::io::Error) -> Self {
        TardisError::io_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<Utf8Error> for TardisError {
    fn from(error: Utf8Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<FromUtf8Error> for TardisError {
    fn from(error: FromUtf8Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<url::ParseError> for TardisError {
    fn from(error: url::ParseError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<ParseIntError> for TardisError {
    fn from(error: ParseIntError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<Infallible> for TardisError {
    fn from(error: Infallible) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<base64::DecodeError> for TardisError {
    fn from(error: base64::DecodeError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<hex::FromHexError> for TardisError {
    fn from(error: hex::FromHexError) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}

impl From<regex::Error> for TardisError {
    fn from(error: regex::Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Basic] {}", error), "")
    }
}
