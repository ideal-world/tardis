use std::env;

pub mod config;
#[cfg(feature = "crypto")]
#[cfg_attr(docsrs, doc(cfg(feature = "crypto")))]
pub mod crypto;
pub mod dto;
pub mod error;
pub mod field;
pub mod json;
pub mod logger;
pub mod result;
pub mod uri;

pub fn fetch_profile() -> String {
    env::var("PROFILE").unwrap_or_else(|_| "".to_string())
}
