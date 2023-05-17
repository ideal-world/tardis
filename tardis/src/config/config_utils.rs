use config::ConfigError;
#[allow(dead_code)]
pub fn config_foreign_err(error: impl std::error::Error + Send + Sync + 'static) -> ConfigError {
    ConfigError::Foreign(Box::new(error))
}
