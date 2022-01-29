use std::env;

pub mod config;
pub mod error;
pub mod field;
pub mod json;
pub mod logger;
pub mod result;
pub mod security;
pub mod uri;
pub mod dto;

pub fn fetch_profile() -> String {
    env::var("PROFILE").unwrap_or("test".to_string())
}
