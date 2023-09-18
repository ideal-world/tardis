use serde::{Deserialize, Serialize};
use tracing_subscriber::filter::Directive;
use typed_builder::TypedBuilder;

use crate::basic::error::TardisError;

use self::tracing_appender::TracingAppenderConfig;
#[cfg(feature = "tracing")]
mod tracing;
#[cfg(feature = "tracing")]
pub use tracing::*;
#[cfg(feature = "tracing-appender")]
mod tracing_appender;
#[cfg(feature = "tracing-appender")]
pub use tracing::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct LogConfig {
    #[builder(default = "info".parse::<Directive>().expect(""), setter(into))]
    #[serde(deserialize_with = "deserialize_directive", serialize_with = "serialize_directive")]
    pub level: Directive,
    #[builder(default, setter(into))]
    #[serde(deserialize_with = "deserialize_directives", serialize_with = "serialize_directives")]
    pub directives: Vec<Directive>,
    #[cfg(feature = "tracing")]
    #[builder(default)]
    pub tracing: TracingConfig,
    #[cfg(feature = "tracing-appender")]
    #[builder(default)]
    pub tracing_appender: TracingAppenderConfig,
}

fn serialize_directive<S>(value: &Directive, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value.to_string().serialize(serializer)
}

fn deserialize_directive<'de, D>(deserializer: D) -> Result<Directive, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.parse::<Directive>().unwrap_or_default())
}
fn serialize_directives<S>(value: &[Directive], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let directives: Vec<String> = value.iter().map(|d| d.to_string()).collect();
    directives.serialize(serializer)
}

fn deserialize_directives<'de, D>(deserializer: D) -> Result<Vec<Directive>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Vec::<String>::deserialize(deserializer)?
        .iter()
        .filter_map(|s| s.parse::<Directive>().map_err(|e| TardisError::internal_error(&format!("update_log_level_by_domain_code failed: {e:?}"), "")).ok())
        .collect())
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig::builder().build()
    }
}
