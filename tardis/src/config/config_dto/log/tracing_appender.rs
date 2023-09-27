use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing_appender::rolling::Rotation;
use typed_builder::TypedBuilder;
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, TypedBuilder, Default)]
pub struct TracingAppenderConfig {
    #[builder(default, setter(into))]
    pub rotation: TracingAppenderRotation,
    pub dir: PathBuf,
    pub filename: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TracingAppenderRotation {
    #[default]
    Never,
    Minutely,
    Hourly,
    Daily,
}

impl From<TracingAppenderRotation> for Rotation {
    fn from(val: TracingAppenderRotation) -> Self {
        match val {
            TracingAppenderRotation::Never => Rotation::NEVER,
            TracingAppenderRotation::Minutely => Rotation::MINUTELY,
            TracingAppenderRotation::Hourly => Rotation::HOURLY,
            TracingAppenderRotation::Daily => Rotation::DAILY,
        }
    }
}
