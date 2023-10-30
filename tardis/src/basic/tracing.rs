use std::sync::Once;

use crate::basic::result::TardisResult;
use crate::config::config_dto::LogConfig;

#[allow(unused_imports)]
use crate::consts::*;
use crate::TARDIS_INST;
pub use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
#[allow(unused_imports)]
use tracing_subscriber::{fmt::Layer as FmtLayer, layer::SubscriberExt, prelude::*, reload::Layer as ReloadLayer, Registry};

/// # Tardis Tracing
/// Tardis tracing is a wrapper of tracing-subscriber. It provides configurable layers as runtime.
///
/// To initialize the tracing, use [TardisTracingInitializer].
///
/// To update config at runtime, use method [`TardisTracing::update_config`].
///
#[derive(Default)]
pub struct TardisTracing<C = LogConfig> {
    configer: Vec<Box<dyn Fn(&C) -> TardisResult<()> + Send + Sync>>,
}

// create a configurable layer, recieve a layer and a configer, return a reload layer and a config function
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

impl<L> TardisTracingInitializer<L>
where
    L: SubscriberInitExt + 'static,
{
    pub fn init(self) {
        static INITIALIZED: Once = Once::new();
        let configer_list = self.configers;
        if INITIALIZED.is_completed() {
            tracing::error!("[Tardis.Tracing] Trying to initialize tardis tracing more than once, this initialization will be ignored. If you want to use new config for tracing, use update_config() instead.");
        } else {
            INITIALIZED.call_once(|| self.layered.init());
            TARDIS_INST.tracing.set(TardisTracing { configer: configer_list });
        }
    }
}

impl TardisTracing<LogConfig> {
    /// Get an initializer for tardis tracing
    pub fn init() -> TardisTracingInitializer<Registry, LogConfig> {
        TardisTracingInitializer::default()
    }

    /// Update tardis tracing config, and this will reload all configurable layers
    /// LogConfig
    pub fn update_config(&self, config: &LogConfig) -> TardisResult<()> {
        for configer in &self.configer {
            (configer)(config)?
        }
        tracing::debug!("[Tardis.Tracing] Config updated.");
        tracing::trace!("[Tardis.Tracing] New config: {:?}", config);
        Ok(())
    }

    pub(crate) fn init_default() -> TardisResult<()> {
        tracing::info!("[Tardis.Tracing] Initializing by defualt initializer.");
        let initializer = TardisTracingInitializer::default();
        let initializer = initializer.with_layer(FmtLayer::default());
        let initializer = initializer.with_configurable_layer(EnvFilter::from_default_env(), |config: &LogConfig| {
            let mut env_filter = EnvFilter::from_default_env();
            env_filter = env_filter.add_directive(config.level.clone());
            for directive in &config.directives {
                env_filter = env_filter.add_directive(directive.clone());
            }
            std::env::set_var("RUST_LOG", env_filter.to_string());
            Ok(env_filter)
        })?;
        tracing::debug!("[Tardis.Tracing] Added fmt layer and env filter.");
        #[cfg(feature = "tracing")]
        let initializer = {
            let initializer = initializer.with_configurable_layer(tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?), |conf: &LogConfig| {
                if std::env::var_os(OTEL_EXPORTER_OTLP_ENDPOINT).is_none() {
                    std::env::set_var(OTEL_EXPORTER_OTLP_ENDPOINT, conf.tracing.endpoint.as_str());
                }
                if std::env::var_os(OTEL_EXPORTER_OTLP_PROTOCOL).is_none() {
                    std::env::set_var(OTEL_EXPORTER_OTLP_PROTOCOL, conf.tracing.protocol.to_string());
                }
                if std::env::var_os(OTEL_SERVICE_NAME).is_none() {
                    std::env::set_var(OTEL_SERVICE_NAME, conf.tracing.server_name.as_str());
                }
                Ok(tracing_opentelemetry::layer().with_tracer(Self::create_otlp_tracer()?))
            })?;
            tracing::debug!("[Tardis.Tracing] Added fmt layer and env filter.");
            initializer
        };
        #[cfg(feature = "console-subscriber")]
        let initializer = {
            use console_subscriber::ConsoleLayer;
            tracing::info!("[Tardis.Tracing] Initializing console subscriber. To make it work, you need to enable tokio and runtime tracing targets at **TRACE** level.");
            let layer = ConsoleLayer::builder().with_default_env().spawn();
            initializer.with_layer(layer)
        };
        #[cfg(feature = "tracing-appender")]
        let initializer = {
            use crate::config::config_dto::log::TracingAppenderConfig;
            let config_file_layer = |cfg: Option<&TracingAppenderConfig>| {
                if let Some(cfg) = &cfg {
                    let file_appender = tracing_appender::rolling::RollingFileAppender::new(cfg.rotation.into(), &cfg.dir, &cfg.filename);
                    FmtLayer::default().with_writer(file_appender).boxed()
                } else {
                    FmtLayer::default().with_writer(std::io::sink).boxed()
                }
            };
            initializer.with_configurable_layer(config_file_layer(None), move |cfg| TardisResult::Ok(config_file_layer(cfg.tracing_appender.as_ref())))?
        };
        tracing::info!("[Tardis.Tracing] Initialize finished.");
        initializer.init();
        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn create_otlp_tracer() -> TardisResult<opentelemetry::sdk::trace::Tracer> {
        use opentelemetry_otlp::WithExportConfig;

        use crate::config::config_dto::OtlpProtocol;
        tracing::debug!("[Tardis.Tracing] Initializing otlp tracer");
        let protocol = std::env::var(OTEL_EXPORTER_OTLP_PROTOCOL).ok().map(|s| s.parse::<OtlpProtocol>()).transpose()?.unwrap_or_default();
        let mut tracer = opentelemetry_otlp::new_pipeline().tracing();
        match protocol {
            OtlpProtocol::Grpc => {
                let mut exporter = opentelemetry_otlp::new_exporter().tonic().with_env();
                // Check if we need TLS
                if let Ok(endpoint) = std::env::var(OTEL_EXPORTER_OTLP_ENDPOINT) {
                    if endpoint.to_lowercase().starts_with("https") {
                        exporter = exporter.with_tls_config(Default::default());
                    }
                }
                tracer = tracer.with_exporter(exporter)
            }
            OtlpProtocol::HttpProtobuf => {
                let headers = Self::parse_otlp_headers_from_env();
                let exporter = opentelemetry_otlp::new_exporter().http().with_headers(headers.into_iter().collect()).with_env();
                tracer = tracer.with_exporter(exporter)
            }
        };
        tracing::debug!("[Tardis.Tracing] Batch installing tracer. If you are blocked here, try running tokio in multithread.");
        let tracer = tracer.install_batch(opentelemetry::runtime::Tokio)?;
        tracing::debug!("[Tardis.Tracing] Initialized otlp tracer");
        Ok(tracer)
    }

    #[cfg(feature = "tracing")]
    fn parse_otlp_headers_from_env() -> Vec<(String, String)> {
        let mut headers = Vec::new();

        if let Ok(hdrs) = std::env::var(OTEL_EXPORTER_OTLP_HEADERS) {
            hdrs.split(',')
                .map(|header| header.split_once('=').expect("Header should contain '=' character"))
                .for_each(|(name, value)| headers.push((name.to_owned(), value.to_owned())));
        }
        headers
    }
}
