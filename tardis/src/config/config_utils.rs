use config::ConfigError;

pub fn config_foreign_err(error: impl std::error::Error + Send + Sync + 'static) -> ConfigError {
    ConfigError::Foreign(Box::new(error))
}