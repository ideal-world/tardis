use std::sync::atomic::{AtomicBool, Ordering};

use crate::basic::result::TardisResult;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{prelude::*, reload::Handle, Registry};

use super::error::TardisError;
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct TardisTracing;

pub static mut GLOBAL_RELOAD_HANDLE: Option<Handle<EnvFilter, Layered<Layer<Registry>, Registry>>> = None;

impl TardisTracing {
    #[cfg(not(feature = "tracing"))]
    pub(crate) fn init_log() -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "info");
        }

        let builder = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).with_filter_reloading();
        let reload_handle = builder.reload_handle();
        builder.finish().init();
        unsafe {
            GLOBAL_RELOAD_HANDLE = Some(reload_handle);
        }
        Ok(())
    }

    pub fn update_log_level(log_level: &str) -> TardisResult<()> {
        std::env::set_var("RUST_LOG", log_level);
        unsafe {
            GLOBAL_RELOAD_HANDLE
                .as_ref()
                .ok_or_else(|| TardisError::internal_error(&format!("{} is none, tracing may not be initialized", stringify!(GLOBAL_RELOAD_HANDLE)), ""))?
                .reload(EnvFilter::from_default_env())?;
        }
        Ok(())
    }

    pub fn update_log_level_by_domain_code(domain_code: &str, log_level: &str) -> TardisResult<()> {
        let env_filter = EnvFilter::from_default_env();
        let env_filter = env_filter
            .add_directive(format!("{domain_code}={log_level}").parse().map_err(|e| TardisError::internal_error(&format!("update_log_level_by_domain_code failed: {e:?}"), ""))?);
        unsafe {
            GLOBAL_RELOAD_HANDLE
                .as_ref()
                .ok_or_else(|| TardisError::internal_error(&format!("{} is none, tracing may not be initialized", stringify!(GLOBAL_RELOAD_HANDLE)), ""))?
                .reload(env_filter)?;
        }
        Ok(())
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(conf: &crate::config::config_dto::FrameworkConfig) -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let builder = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).with_filter_reloading();
        let reload_handle = builder.reload_handle();
        unsafe {
            GLOBAL_RELOAD_HANDLE = Some(reload_handle);
        }
        #[cfg(not(feature = "tardis-trace-exporter"))]
        builder.finish().init();

        #[cfg(feature = "tardis-trace-exporter")]
        {
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
            let telemetry_layer = tracing_opentelemetry::layer().with_tracer( tardis_trace_exporter::create_otlp_tracer().map_err(|e| TardisError::conflict(&e.to_string(), ""))?);
            builder.finish().with(telemetry_layer).init();
        }
        Ok(())
    }
}
