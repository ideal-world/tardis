use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, OnceLock, OnceState};

use crate::basic::result::TardisResult;
use crate::config::config_dto::{FrameworkConfig, LogConfig};
use crate::utils::TardisComponentMap;
use tracing::Subscriber;

use super::error::TardisError;
use std::pin::Pin;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{fmt::Layer as FmtLayer, layer::SubscriberExt, prelude::*, reload::Handle, reload::Layer as ReloadLayer, Registry};

#[derive(Default)]
pub struct TardisTracing<C = LogConfig> {
    configer: Vec<Box<dyn Fn(&C) -> TardisResult<()> + Send + Sync>>,
}

fn create_configurable_layer<L, S, C>(layer: L, configer: impl Fn(&C) -> TardisResult<L> + Send + Sync) -> TardisResult<(ReloadLayer<L, S>, impl Fn(&C) -> TardisResult<()>)> {
    let (reload_layer, reload_handle) = ReloadLayer::new(layer);
    let config_layer_fn = move |conf: &C| -> TardisResult<()> {
        let layer = configer(conf)?;
        reload_handle.reload(layer)?;
        Ok(())
    };
    Ok((reload_layer, config_layer_fn))
}


/// Tardis tracing initializer
/// ```ignore
/// # use crate::basic::tracing::TardisTracingInitializer;
/// # use tracing_subscriber::{fmt, EnvFilter}
/// TardisTracing::init()
///     .with_layer(fmt::layer())
///     .with_configurable_layer(EnvFilter::from_default_env(), |config| {
///         let env_filter = EnvFilter::from_default_env();
///         // handle with config
///         Ok(env_filter)
///     })
///     .init();
/// ```
pub struct TardisTracingInitializer<L, C = LogConfig> {
    /// 所有延迟配置函数
    configers: Vec<Box<dyn Fn(&C) -> TardisResult<()> + Send + Sync>>,
    /// 外部创建层
    layered: L,
}

impl<C: 'static> Default for TardisTracingInitializer<Registry, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static> TardisTracingInitializer<Registry, C> {
    pub fn new() -> Self {
        Self {
            configers: Vec::new(),
            layered: Registry::default(),
        }
    }
}

impl<L0, C: 'static> TardisTracingInitializer<L0, C>
where
    L0: SubscriberExt,
{
    pub fn with_configurable_layer<L, S>(
        mut self,
        layer: L,
        configer: impl Fn(&C) -> TardisResult<L> + 'static + Send + Sync,
    ) -> TardisResult<TardisTracingInitializer<Layered<ReloadLayer<L, S>, L0>, C>>
    where
        ReloadLayer<L, S>: tracing_subscriber::Layer<L0>,
        L: 'static + Send + Sync,
    {
        let (reload_layer, config_layer_fn) = create_configurable_layer::<L, S, C>(layer, configer)?;
        self.configers.push(Box::new(config_layer_fn));
        Ok(TardisTracingInitializer {
            configers: self.configers,
            layered: self.layered.with(reload_layer),
        })
    }

    pub fn with_layer<L>(self, layer: L) -> TardisTracingInitializer<Layered<L, L0>, C>
    where
        L: tracing_subscriber::Layer<L0>,
    {
        TardisTracingInitializer {
            configers: self.configers,
            layered: self.layered.with(layer),
        }
    }
}

impl<L, C: 'static> TardisTracingInitializer<L, C>
where
    L: SubscriberInitExt + 'static,
{
    pub fn init(self) -> TardisTracing<C> {
        static INITIALIZED: Once = Once::new();
        let configer_list = self.configers;
        INITIALIZED.call_once(|| self.layered.init());
        TardisTracing { configer: configer_list }
    }
}

impl TardisTracing<LogConfig> {
    /// Get an initializer for tardis tracing
    pub fn init() -> TardisTracingInitializer<Registry, LogConfig> {
        TardisTracingInitializer::default()
    }

    /// Update tardis tracing config, and this will reload all configurable layers
    /// 
    pub fn update_config(&self, config: &LogConfig) -> TardisResult<()> {
        for configer in &self.configer {
            (configer)(config)?
        }
        Ok(())
    }


    pub(crate) fn init_default() -> TardisResult<Self> {
        let initializer = TardisTracingInitializer::default().with_layer(FmtLayer::default()).with_configurable_layer(EnvFilter::from_default_env(), |config: &LogConfig| {
            let mut env_filter = EnvFilter::from_default_env();
            for directive in &config.directives {
                env_filter = env_filter.add_directive(directive.clone());
            }
            std::env::set_var("RUST_LOG", env_filter.to_string());
            Ok(env_filter)
        })?;
        #[cfg(feature = "tracing")]
        let initializer = initializer.with_configurable_layer(tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?), |conf: &LogConfig| {
            if std::env::var_os("RUST_LOG").is_none() {
                std::env::set_var("RUST_LOG", conf.level.as_str());
            }
            if std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_none() {
                std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", conf.endpoint.as_str());
            }
            if std::env::var_os("OTEL_EXPORTER_OTLP_PROTOCOL").is_none() {
                std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", conf.protocol.as_str());
            }
            if std::env::var_os("OTEL_SERVICE_NAME").is_none() {
                std::env::set_var("OTEL_SERVICE_NAME", conf.server_name.as_str());
            }
            Ok(tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?))
        })?;
        let tardis_tracing = initializer.init();
        Ok(tardis_tracing)
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
