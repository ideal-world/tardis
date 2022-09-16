use std::env;

pub mod dto;
pub mod error;
pub mod field;
pub mod json;
pub mod locale;
pub mod logger;
pub mod result;
pub mod uri;

pub fn fetch_profile() -> String {
    env::var("PROFILE").unwrap_or_else(|_| "".to_string())
}
