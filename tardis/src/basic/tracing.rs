use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::basic::result::TardisResult;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, prelude::*, reload::Handle, Registry};

use super::error::TardisError;
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub trait Initializer<T: ?Sized> {
    fn setup(&self, target: &mut T);
}

pub trait TardisTracingInitializer {}
pub struct TardisTracing {
    layer_modifiers: Vec<Box<dyn Initializer<dyn SubscriberExt>>>,
}

pub static GLOBAL_RELOAD_HANDLE: OnceLock<Handle<EnvFilter, Layered<Layer<Registry>, Registry>>> = OnceLock::new();

impl TardisTracing {
    /// initialize the log layer
    /// ```plaintext
    /// +---------+
    /// |  relaod |
    /// | +-------+
    /// | |   env |
    /// | | +-----+
    /// | | | fmt |
    /// +-+-+-----+
    /// ```
    ///
    ///
    pub(crate) fn init_log() -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "info");
        }

        let builder = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).with_filter_reloading();
        let fmt_layer = tracing_subscriber::fmt::layer();
        let reload_handle = builder.reload_handle();

        let registry = Registry::default().with(fmt_layer).with(EnvFilter::from_default_env());
        // let (layer, reload_handle): (_, Handle<EnvFilter, Layered<Layer<Registry>, Registry>>) = tracing_subscriber::reload::Layer::new(registry);
        let fmt_sub = builder.finish().init();
        GLOBAL_RELOAD_HANDLE.get_or_init(|| reload_handle);
        Ok(())
    }

    pub fn update_log_level(log_level: &str) -> TardisResult<()> {
        std::env::set_var("RUST_LOG", log_level);
        GLOBAL_RELOAD_HANDLE
            .get()
            .ok_or_else(|| TardisError::internal_error(&format!("{} is none, tracing may not be initialized", stringify!(GLOBAL_RELOAD_HANDLE)), ""))?
            .reload(EnvFilter::from_default_env())?;
        Ok(())
    }

    pub fn update_log_level_by_domain_code(domain_code: &str, log_level: &str) -> TardisResult<()> {
        let env_filter = EnvFilter::from_default_env();
        let env_filter = env_filter
            .add_directive(format!("{domain_code}={log_level}").parse().map_err(|e| TardisError::internal_error(&format!("update_log_level_by_domain_code failed: {e:?}"), ""))?);
        std::env::set_var("RUST_LOG", env_filter.to_string());
        GLOBAL_RELOAD_HANDLE
            .get()
            .ok_or_else(|| TardisError::internal_error(&format!("{} is none, tracing may not be initialized", stringify!(GLOBAL_RELOAD_HANDLE)), ""))?
            .reload(env_filter)?;
        Ok(())
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(conf: &crate::config::config_dto::FrameworkConfig) -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(tracing_config) = conf.log.as_ref() {
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
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?);
        let builder = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).with_filter_reloading();
        let reload_handle = builder.reload_handle();
        GLOBAL_RELOAD_HANDLE.get_or_init(|| reload_handle);
        builder.finish().with(telemetry_layer).init();
        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn create_otlp_tracer() -> TardisResult<opentelemetry::sdk::trace::Tracer> {
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
        Ok(tracer.install_batch(opentelemetry::runtime::Tokio)?)
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
