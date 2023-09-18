use std::str::FromStr;

use serde::{Deserialize, Serialize};

use typed_builder::TypedBuilder;

use crate::basic::error::TardisError;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum OtlpProtocol {
    #[default]
    Grpc,
    HttpProtobuf,
}

impl ToString for OtlpProtocol {
    fn to_string(&self) -> String {
        match self {
            OtlpProtocol::Grpc => "grpc".to_string(),
            OtlpProtocol::HttpProtobuf => "http/protobuf".to_string(),
        }
    }
}

impl FromStr for OtlpProtocol {
    type Err = TardisError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "grpc" => Ok(OtlpProtocol::Grpc),
            "http/protobuf" => Ok(OtlpProtocol::HttpProtobuf),
            _ => Err(TardisError::conflict(&format!("[Tracing] Unsupported protocol {s}"), "")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct TracingConfig {
    #[cfg(feature = "tracing")]
    #[builder(default = "http://localhost:4317".to_string(), setter(into))]
    pub endpoint: String,
    #[cfg(feature = "tracing")]
    #[builder(default)]
    pub protocol: OtlpProtocol,
    #[cfg(feature = "tracing")]
    #[builder(default = "tardis-tracing".to_string(), setter(into))]
    pub server_name: String,
    #[cfg(feature = "tracing")]
    #[builder(default, setter(into, strip_option))]
    pub headers: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}
