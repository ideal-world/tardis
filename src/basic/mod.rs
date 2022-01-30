use std::env;

pub mod config;
pub mod dto;
pub mod error;
pub mod field;
pub mod json;
pub mod logger;
pub mod result;
pub mod security;
pub mod uri;

pub fn fetch_profile() -> String {
    env::var("PROFILE").unwrap_or_else(|_| "test".to_string())
}
