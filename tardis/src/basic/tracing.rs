use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::basic::result::TardisResult;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{prelude::*, reload::Handle, Registry};
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct TardisTracing;

pub static mut GLOBAL_RELOAD_HANDLE: Option<Handle<LevelFilter, Registry>> = None;

impl TardisTracing {
    #[cfg(not(feature = "tracing"))]
    pub(crate) fn init_log() -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let level = std::env::var_os("RUST_LOG").unwrap_or(std::ffi::OsString::from("info")).into_string().unwrap();
        let subscriber = tracing_subscriber::registry();
        if level.split_once(',').is_some() {
            subscriber.with(tracing_subscriber::fmt::Layer::default()).init();
        } else {
            let filter = tracing_subscriber::filter::LevelFilter::from_str(level.as_str()).unwrap();
            let (filter, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
            tracing_subscriber::registry().with(filter).with(tracing_subscriber::fmt::Layer::default()).init();
            unsafe {
                GLOBAL_RELOAD_HANDLE = Some(reload_handle);
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "tracing"))]
    pub(crate) fn update_log_level(log_level: &str) -> TardisResult<()> {
        unsafe {
            GLOBAL_RELOAD_HANDLE.as_ref().unwrap().modify(|filter| *filter = tracing_subscriber::filter::LevelFilter::from_str(log_level).unwrap()).unwrap();
        }
        Ok(())
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(conf: &crate::config::config_dto::TardisConfig) -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
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
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?);
        tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::from_default_env()).with(fmt_layer).with(telemetry_layer).init();
        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn create_otlp_tracer() -> TardisResult<opentelemetry::sdk::trace::Tracer> {
        use crate::basic::error::TardisError;
        use opentelemetry_otlp::WithExportConfig;

        let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or("grpc".to_string());
        let mut tracer = opentelemetry_otlp::new_pipeline().tracing();
        match protocol.as_str() {
            "grpc" => {
                let mut exporter = opentelemetry_otlp::new_exporter().tonic().with_env();
                // Check if we need TLS
                if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
                    if endpoint.to_lowercase().starts_with("https") {
                        exporter = exporter.with_tls_config(Default::default());
                    }
                }
                tracer = tracer.with_exporter(exporter)
            }
            "http/protobuf" => {
                let headers = Self::parse_otlp_headers_from_env();
                let exporter = opentelemetry_otlp::new_exporter().http().with_headers(headers.into_iter().collect()).with_env();
                tracer = tracer.with_exporter(exporter)
            }
            p => return Err(TardisError::conflict(&format!("[Tracing] Unsupported protocol {p}"), "")),
        };
        Ok(tracer.install_batch(opentelemetry::runtime::Tokio).unwrap())
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
}
