use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing_subscriber::filter::Directive;
use typed_builder::TypedBuilder;

#[cfg(feature = "tracing")]
mod tracing;
#[cfg(feature = "tracing")]
pub use tracing::*;
#[cfg(feature = "tracing-appender")]
mod tracing_appender;
#[cfg(feature = "tracing-appender")]
pub use tracing_appender::*;

/// # Log configure
///
/// - level: global log level, default to `info`
/// - directives: log level with targets and modules, e.g. `tardis=debug,sqlx=info`
/// ## Example
/// ```toml
/// [fw.log]
/// level = "info"
/// directives = ["tardis=debug", "sqlx=info"]
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct LogConfig {
    #[builder(default, setter(into))]
    #[serde(deserialize_with = "deserialize_directive", serialize_with = "serialize_directive")]
    /// the default log level
    pub level: Option<Directive>,
    #[builder(default, setter(into))]
    #[serde(deserialize_with = "deserialize_directives", serialize_with = "serialize_directives")]
    /// tracing filtering directive, e.g. `tardis=debug,sqlx=off`
    pub directives: Vec<Directive>,
    #[cfg(feature = "tracing")]
    #[builder(!default, default = None)]
    /// open telemetry tracing config
    pub tracing: Option<TracingConfig>,
    #[cfg(feature = "tracing-appender")]
    #[builder(default)]
    /// tracing appender config
    /// a `None` value means no file output
    pub tracing_appender: Option<TracingAppenderConfig>,
    /// extension config for custom tracing layers
    #[builder(default)]
    pub ext: HashMap<String, crate::serde_json::Value>,
}

fn serialize_directive<S>(value: &Option<Directive>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value.as_ref().map(|d| d.to_string()).serialize(serializer)
}

fn deserialize_directive<'de, D>(deserializer: D) -> Result<Option<Directive>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <Option<String>>::deserialize(deserializer)?.map(|s| s.parse::<Directive>().map_err(serde::de::Error::custom)).transpose()
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
    let directives: Vec<String> = Vec::deserialize(deserializer)?;
    directives.into_iter().map(|d| d.parse::<Directive>().map_err(serde::de::Error::custom)).collect()
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig::builder().build()
    }
}
