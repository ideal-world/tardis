use std::collections::HashMap;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::sync::Mutex;

use log::info;
use regex::Regex;

use crate::basic::error::TardisError;
use crate::{TardisFuns, TardisResult};

lazy_static! {
    static ref LOCALE_CONFIG: Mutex<HashMap<String, HashMap<String, (String, Option<Regex>)>>> = Mutex::new(HashMap::new());
}

pub struct TardisLocale;

impl TardisLocale {
    pub(crate) fn init(path: &Path) -> TardisResult<()> {
        let path = path.join("locale");
        if !path.exists() {
            return Ok(());
        }
        info!("[Tardis.Locale] Initializing, base path:{:?}", path);
        let mut conf = LOCALE_CONFIG.lock().map_err(|error| TardisError::internal_error(&format!("{error:?}"), ""))?;
        let paths = path.read_dir().map_err(|error| TardisError::bad_request(&format!("[Tardis.Locale] Path {path:#?} dir error:{error:#?}"), ""))?;
        for entry in paths {
            let entry = entry?;
            let name = entry.file_name();
            let lang = name
                .to_str()
                .ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Locale] File name error {entry:#?}"), ""))?
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
                let code = items.next().ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Locale] Code not exist in {line}"), ""))?.trim();
                let message = items.next().ok_or_else(|| TardisError::bad_request(&format!("[Tardis.Locale] Message not exist in {line}"), ""))?.trim();
                let regex = if let Some(regex) = items.next() {
                    Some(Regex::new(regex.trim()).map_err(|_| TardisError::bad_request(&format!("[Tardis.Locale] Regex illegal in {line}"), ""))?)
                } else {
                    None
                };
                conf.entry(lang.to_string()).or_insert_with(HashMap::new).insert(code.to_string(), (message.to_string(), regex));
            }
        }
        Ok(())
    }

    pub fn get_message(code: &str, default_message: &str, lang: &str) -> TardisResult<String> {
        let lang = lang.to_lowercase();
        let conf = LOCALE_CONFIG.lock().map_err(|error| TardisError::conflict(&format!("[Tardis.Locale] locale config lock error: {error:?}"), ""))?;
        if let Some(conf) = conf.get(&lang) {
            if let Some((message, regex)) = conf.get(code) {
                let mut localized_message = message.clone();
                if let Some(regex) = regex {
                    return if let Some(cap) = regex.captures(default_message) {
                        for (idx, cap) in cap.iter().enumerate() {
                            if let Some(cap) = cap {
                                localized_message = localized_message.replace(&format!("{{{idx}}}"), cap.as_str());
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

    pub fn env_message(code: &str, default_message: &str) -> String {
        if let Some(lang) = &TardisFuns::default_lang() {
            TardisLocale::get_message(code, default_message, lang).unwrap_or_else(|_| default_message.to_string())
        } else {
            default_message.to_string()
        }
    }
}
