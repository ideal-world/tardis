use std::sync::atomic::{AtomicBool, Ordering};

use crate::basic::result::TardisResult;
#[cfg(feature = "tracing")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct TardisLogger;

impl TardisLogger {
    pub(crate) fn init() -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "info");
        }
        if std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_none() {
            std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
        }
        if std::env::var_os("OTEL_EXPORTER_OTLP_PROTOCOL").is_none() {
            std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc");
        }
        if std::env::var_os("OTEL_SERVICE_NAME").is_none() {
            std::env::set_var("OTEL_SERVICE_NAME", "tardis-tracing");
        }
        #[cfg(not(feature = "tracing"))]
        {
            tracing_subscriber::fmt::init();
        }
        #[cfg(feature = "tracing")]
        {
            Self::init_tracing().unwrap();
        }
        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn init_tracing() -> TardisResult<()> {
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
