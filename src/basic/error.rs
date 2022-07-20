use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::num::ParseIntError;
use std::path::Path;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Mutex;

use derive_more::Display;
use log::{info, warn};
use regex::Regex;

use crate::basic::field::GENERAL_SPLIT;
use crate::TardisResult;

pub static ERROR_DEFAULT_CODE: &str = "-1";

lazy_static! {
    static ref LOCALE_CONFIG: Mutex<HashMap<String, HashMap<String, (String, Option<Regex>)>>> = Mutex::new(HashMap::new());
}

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
    pub fn init_locale(path: &Path) -> TardisResult<()> {
        let path = path.join("locale");
        if !path.exists() {
            return Ok(());
        }
        info!("[Tardis.Error] Initializing, base path:{:?}", path);
        let mut conf = LOCALE_CONFIG.lock().map_err(|e| TardisError::InternalError(format!("{:?}", e)))?;
        let paths = path.read_dir().map_err(|e| TardisError::BadRequest(format!("[Tardis.Error] path {:#?} dir error:{:#?}", path, e)))?;
        for entry in paths {
            let entry = entry?;
            let name = entry.file_name();
            let lang = name
                .to_str()
                .ok_or_else(|| TardisError::BadRequest(format!("[Tardis.Error] file name error {:#?}", entry)))?
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
                let code = items.next().ok_or_else(|| TardisError::BadRequest(format!("[Tardis.Error] code not exist in {}", line)))?.trim();
                let message = items.next().ok_or_else(|| TardisError::BadRequest(format!("[Tardis.Error] message not exist in {}", line)))?.trim();
                let regex = if let Some(regex) = items.next() {
                    Some(Regex::new(regex.trim()).map_err(|_| TardisError::BadRequest(format!("[Tardis.Error] regex illegal in {}", line)))?)
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
        let conf = LOCALE_CONFIG.lock().map_err(|e| TardisError::Conflict(format!("[Tardis.Error] locale config lock error: {:?}", e)))?;
        if let Some(conf) = conf.get(&lang) {
            if let Some((message, regex)) = conf.get(code) {
                let mut localized_message = message.clone();
                if let Some(regex) = regex {
                    if let Some(cap) = regex.captures(default_message) {
                        for (idx, cap) in cap.iter().enumerate() {
                            if let Some(cap) = cap {
                                localized_message = localized_message.replace(&format!("{{{}}}", idx), cap.as_str());
                            }
                        }
                        return Ok(localized_message);
                    } else {
                        // Regex not match, fallback to default message
                        return Ok(default_message.to_string());
                    }
                }
                // No regex, return default message
                return Ok(message.to_string());
            }
        }
        // No locale config, return default message
        Ok(default_message.to_string())
    }

    pub fn form(msg: &str) -> TardisError {
        let (code, message) = Self::to_tuple(msg.to_string());
        TardisError::Custom(code, message)
    }

    fn to_tuple(msg: String) -> (String, String) {
        let split_idx = msg.find(GENERAL_SPLIT).expect("[Tardis.Error] illegal error description format");
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
        let split_idx = text.find(GENERAL_SPLIT).expect("[Tardis.Error] illegal code description format");
        let code = &text[..split_idx];
        code.to_string()
    }

    pub fn message(&self) -> String {
        let text = self.to_string();
        let split_idx = text.find(GENERAL_SPLIT).expect("[Tardis.Error] illegal message description format");
        let message = &text[split_idx + 2..];
        message.to_string()
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
        let msg = self.localized_message(if locale_code.trim().is_empty() { &code } else { locale_code }, msg);
        TardisError::Custom(code, msg)
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
