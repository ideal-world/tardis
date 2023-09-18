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
    /// tracing appender config
    /// a `None` value means no file output
    pub tracing_appender: Option<TracingAppenderConfig>,
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
    String::deserialize(deserializer)?.parse::<Directive>().map_err(serde::de::Error::custom)
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
