use std::str::FromStr;
use std::{
    ffi::OsString,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{basic::result::TardisResult, config::config_dto::TardisConfig};
#[cfg(feature = "tracing")]
use opentelemetry_otlp::WithExportConfig;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    filter, fmt,
    prelude::*,
    reload::{self, Handle},
    Registry,
};

static INITIALIZED_TRACING: AtomicBool = AtomicBool::new(false);
static INITIALIZED_LOG: AtomicBool = AtomicBool::new(false);

pub struct TardisTracing;

pub static mut global_reload_handle: Option<Handle<LevelFilter, Registry>> = None;

impl TardisTracing {
    #[cfg(not(feature = "tracing"))]
    pub(crate) fn init_log() -> TardisResult<()> {
        if INITIALIZED_LOG.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let level = std::env::var_os("RUST_LOG").unwrap_or(OsString::from("info")).into_string().unwrap();
        let filter = filter::LevelFilter::from_str(level.as_str()).unwrap();
        let (filter, reload_handle) = reload::Layer::new(filter);
        tracing_subscriber::registry().with(filter).with(fmt::Layer::default()).init();
        unsafe {
            global_reload_handle = Some(reload_handle);
        }

        Ok(())
    }
    #[cfg(not(feature = "tracing"))]
    pub(crate) fn update_log_level(log_level: &str) -> TardisResult<()> {
        unsafe {
            global_reload_handle.as_ref().unwrap().modify(|filter| *filter = filter::LevelFilter::from_str(log_level).unwrap()).unwrap();
        }
        Ok(())
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(conf: &TardisConfig) -> TardisResult<()> {
        if INITIALIZED_TRACING.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(tracing_config) = conf.fw.log.as_ref() {
            if std::env::var_os("RUST_LOG").is_none() {
                std::env::set_var("RUST_LOG", tracing_config.level.as_str());
            }
            if std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_none() {
                std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", tracing_config.endpoint.as_str());
            }
            if std::env::var_os("OTEL_EXPORTER_OTLP_PROTOCOL").is_none() {
                std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", tracing_config.protocol.as_str());
            }
            if std::env::var_os("OTEL_SERVICE_NAME").is_none() {
                std::env::set_var("OTEL_SERVICE_NAME", tracing_config.server_name.as_str());
            }
        }
        let fmt_layer = tracing_subscriber::fmt::layer();

        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer());

        tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::from_default_env()).with(fmt_layer).with(telemetry_layer).init();

        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn create_otlp_tracer() -> opentelemetry::sdk::trace::Tracer {
        let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or("grpc".to_string());

        let mut tracer = opentelemetry_otlp::new_pipeline().tracing();
        let headers = Self::parse_otlp_headers_from_env();

        match protocol.as_str() {
            "grpc" => {
                let mut exporter = opentelemetry_otlp::new_exporter().tonic().with_metadata(Self::metadata_from_headers(headers)).with_env();

                // Check if we need TLS
                if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
                    if endpoint.starts_with("https") {
                        exporter = exporter.with_tls_config(Default::default());
                    }
                }
                tracer = tracer.with_exporter(exporter)
            }
            "http/protobuf" => {
                let exporter = opentelemetry_otlp::new_exporter().http().with_headers(headers.into_iter().collect()).with_env();
                tracer = tracer.with_exporter(exporter)
            }
            p => panic!("Unsupported protocol {}", p),
        };

        tracer.install_batch(opentelemetry::runtime::Tokio).unwrap()
    }

    #[cfg(feature = "tracing")]
    fn parse_otlp_headers_from_env() -> Vec<(String, String)> {
        let mut headers = Vec::new();

        if let Ok(hdrs) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
            hdrs.split(',')
                .map(|header| header.split_once('=').expect("Header should contain '=' character"))
                .for_each(|(name, value)| headers.push((name.to_owned(), value.to_owned())));
        }
        headers
    }

    #[cfg(feature = "tracing")]
    fn metadata_from_headers(headers: Vec<(String, String)>) -> tonic::metadata::MetadataMap {
        use std::str::FromStr;
        use tonic::metadata;

        let mut metadata = metadata::MetadataMap::new();
        headers.into_iter().for_each(|(name, value)| {
            let value = value.parse::<metadata::MetadataValue<metadata::Ascii>>().expect("Header value invalid");
            metadata.insert(metadata::MetadataKey::from_str(&name).unwrap(), value);
        });
        metadata
    }
}
