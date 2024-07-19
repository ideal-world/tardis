use std::sync::{Arc, Once};

use crate::basic::result::TardisResult;
use crate::config::config_dto::LogConfig;

#[allow(unused_imports)]
use crate::consts::*;
use crate::tardis_instance;
pub use tracing_subscriber::filter::Directive;
#[allow(unused_imports)]
use tracing_subscriber::{
    fmt::Layer as FmtLayer,
    layer::{Layered, SubscriberExt},
    prelude::*,
    registry::LookupSpan,
    reload::Layer as ReloadLayer,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// # Tardis Tracing
/// Tardis tracing is a wrapper of tracing-subscriber. It provides configurable layers as runtime.
///
/// To initialize the tracing, use [TardisTracingInitializer].
///
/// To update config at runtime, use method [`TardisTracing::update_config`].
///
#[derive(Default)]
pub struct TardisTracing<C = LogConfig> {
    configure: Vec<Box<dyn Fn(&C) -> TardisResult<()> + Send + Sync>>,
}

// create a configurable layer, recieve a layer and a configer, return a reload layer and a config function
fn create_configurable_layer<L, S, C>(layer: L, configer: impl Fn(&C) -> TardisResult<L> + Send + Sync) -> (ReloadLayer<L, S>, impl Fn(&C) -> TardisResult<()>) {
    let (reload_layer, reload_handle) = ReloadLayer::new(layer);
    let config_layer_fn = move |conf: &C| -> TardisResult<()> {
        let layer = configer(conf)?;
        reload_handle.reload(layer)?;
        Ok(())
    };
    (reload_layer, config_layer_fn)
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
    ) -> TardisTracingInitializer<Layered<ReloadLayer<L, S>, L0>, C>
    where
        ReloadLayer<L, S>: tracing_subscriber::Layer<L0>,
        L: 'static + Send + Sync,
    {
        let (reload_layer, config_layer_fn) = create_configurable_layer::<L, S, C>(layer, configer);
        self.configers.push(Box::new(config_layer_fn));
        TardisTracingInitializer {
            configers: self.configers,
            layered: self.layered.with(reload_layer),
        }
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

type BoxLayer<S> = Box<dyn tracing_subscriber::Layer<S> + Send + Sync + 'static>;
impl<L0> TardisTracingInitializer<L0, LogConfig>
where
    L0: SubscriberExt,
{
    pub fn with_fmt_layer<S>(self) -> TardisTracingInitializer<Layered<BoxLayer<S>, L0>, LogConfig>
    where
        S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
        BoxLayer<S>: tracing_subscriber::Layer<L0>,
    {
        self.with_layer(FmtLayer::default().boxed())
    }
    pub fn with_env_layer<S>(self) -> TardisTracingInitializer<Layered<ReloadLayer<BoxLayer<S>, S>, L0>, LogConfig>
    where
        S: tracing::Subscriber,
        ReloadLayer<BoxLayer<S>, S>: tracing_subscriber::Layer<L0>,
    {
        self.with_configurable_layer(EnvFilter::from_default_env().boxed(), |config: &LogConfig| {
            let mut env_filter = EnvFilter::from_default_env();
            if let Some(level) = config.level.clone() {
                env_filter = env_filter.add_directive(level);
            }
            for directive in &config.directives {
                env_filter = env_filter.add_directive(directive.clone());
            }
            std::env::set_var("RUST_LOG", env_filter.to_string());
            Ok(env_filter.boxed())
        })
    }

    #[cfg(feature = "tracing")]
    pub fn with_opentelemetry_layer<S>(self) -> TardisTracingInitializer<Layered<ReloadLayer<BoxLayer<S>, S>, L0>, LogConfig>
    where
        S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
        ReloadLayer<BoxLayer<S>, S>: tracing_subscriber::Layer<L0>,
    {
        self.with_configurable_layer(
            tracing_opentelemetry::layer().with_tracer(TardisTracing::<LogConfig>::create_otlp_tracer()).boxed(),
            |conf: &LogConfig| {
                if std::env::var_os(OTEL_EXPORTER_OTLP_ENDPOINT).is_none() {
                    std::env::set_var(OTEL_EXPORTER_OTLP_ENDPOINT, conf.tracing.endpoint.as_str());
                }
                if std::env::var_os(OTEL_EXPORTER_OTLP_PROTOCOL).is_none() {
                    std::env::set_var(OTEL_EXPORTER_OTLP_PROTOCOL, conf.tracing.protocol.to_string());
                }
                if std::env::var_os(OTEL_SERVICE_NAME).is_none() {
                    std::env::set_var(OTEL_SERVICE_NAME, conf.tracing.server_name.as_str());
                }
                Ok(tracing_opentelemetry::layer().with_tracer(TardisTracing::<LogConfig>::create_otlp_tracer()).boxed())
            },
        )
    }

    #[cfg(feature = "console-subscriber")]
    pub fn with_console_layer<S>(self) -> TardisTracingInitializer<Layered<BoxLayer<S>, L0>, LogConfig>
    where
        S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
        BoxLayer<S>: tracing_subscriber::Layer<L0>,
    {
        self.with_layer(console_subscriber::ConsoleLayer::builder().with_default_env().spawn().boxed())
    }

    #[cfg(feature = "tracing-appender")]
    pub fn with_appender_layer<S>(self) -> TardisTracingInitializer<Layered<ReloadLayer<BoxLayer<S>, S>, L0>, LogConfig>
    where
        S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
        ReloadLayer<BoxLayer<S>, S>: tracing_subscriber::Layer<L0>,
    {
        use crate::config::config_dto::log::TracingAppenderConfig;
        let config_file_layer = |cfg: Option<&TracingAppenderConfig>| {
            if let Some(cfg) = &cfg {
                let file_appender = tracing_appender::rolling::RollingFileAppender::new(cfg.rotation.into(), &cfg.dir, &cfg.filename);
                FmtLayer::default().with_writer(file_appender).boxed()
            } else {
                FmtLayer::default().with_writer(std::io::sink).boxed()
            }
        };
        self.with_configurable_layer(config_file_layer(None), move |cfg| TardisResult::Ok(config_file_layer(cfg.tracing_appender.as_ref())))
    }
}

impl<L> TardisTracingInitializer<L>
where
    L: SubscriberInitExt + 'static,
{
    /// Initialize tardis tracing, this will set the global tardis tracing instance.
    pub fn init(self) -> Arc<TardisTracing> {
        static INITIALIZED: Once = Once::new();
        let configer_list = self.configers;
        if INITIALIZED.is_completed() {
            tracing::error!("[Tardis.Tracing] Trying to initialize tardis tracing more than once, this initialization will be ignored. If you want to use new config for tracing, use update_config() instead.");
        } else {
            INITIALIZED.call_once(|| self.layered.init());
            tardis_instance().tracing.set(TardisTracing { configure: configer_list });
        }
        crate::TardisFuns::tracing()
    }

    /// Initialize tardis tracing as standalone, this will not set the global tardis tracing instance.
    /// # Warning
    /// Config this standalong instance will also change the value of env variable `RUST_LOG`,
    /// if you are using the global tardis tracing instance, you should use [`TardisTracingInitializer::init`] instead.
    pub fn init_standalone(self) -> TardisTracing {
        let configer_list = self.configers;
        self.layered.init();
        TardisTracing { configure: configer_list }
    }
}

impl TardisTracing<LogConfig> {
    /// Get an initializer for tardis tracing
    pub fn initializer() -> TardisTracingInitializer<Registry, LogConfig> {
        TardisTracingInitializer::default()
    }

    /// Update tardis tracing config, and this will reload all configurable layers
    /// LogConfig
    pub fn update_config(&self, config: &LogConfig) -> TardisResult<()> {
        for configer in &self.configure {
            (configer)(config)?
        }
        tracing::debug!("[Tardis.Tracing] Config updated.");
        tracing::trace!("[Tardis.Tracing] New config: {:?}", config);
        Ok(())
    }

    pub(crate) fn init_default() {
        tracing::info!("[Tardis.Tracing] Initializing by default initializer.");
        let initializer = TardisTracingInitializer::default().with_fmt_layer().with_env_layer();
        tracing::debug!("[Tardis.Tracing] Added fmt layer and env filter.");
        #[cfg(feature = "tracing")]
        let initializer = initializer.with_opentelemetry_layer();
        #[cfg(feature = "tracing-appender")]
        let initializer = initializer.with_appender_layer();
        tracing::info!("[Tardis.Tracing] Initialize finished.");
        initializer.init();
    }

    #[cfg(feature = "tracing")]
    fn create_otlp_tracer() -> opentelemetry_sdk::trace::Tracer {
        use crate::config::config_dto::OtlpProtocol;
        tracing::debug!("[Tardis.Tracing] Initializing otlp tracer");
        let protocol = std::env::var(OTEL_EXPORTER_OTLP_PROTOCOL).ok().map(|s| s.parse::<OtlpProtocol>().unwrap_or_default()).unwrap_or_default();
        let tracer = opentelemetry_otlp::new_pipeline().tracing();
        let tracer = match protocol {
            OtlpProtocol::Grpc => {
                let mut exporter = opentelemetry_otlp::new_exporter().tonic();
                // Check if we need TLS
                if let Ok(endpoint) = std::env::var(OTEL_EXPORTER_OTLP_ENDPOINT) {
                    if endpoint.to_lowercase().starts_with("https") {
                        exporter = exporter.with_tls_config(Default::default());
                    }
                }
                tracer.with_exporter(exporter)
            }
            OtlpProtocol::HttpProtobuf => {
                let headers = Self::parse_otlp_headers_from_env();
                let exporter = opentelemetry_otlp::new_exporter().http().with_headers(headers.into_iter().collect());
                tracer.with_exporter(exporter)
            }
        };
        tracing::debug!("[Tardis.Tracing] Batch installing tracer. If you are blocked here, try running tokio in multithread.");
        let tracer = tracer.install_batch(opentelemetry_sdk::runtime::Tokio).expect("fail to install otlp tracer");
        tracing::debug!("[Tardis.Tracing] Initialized otlp tracer");
        tracer
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

#[cfg(feature = "tracing")]
pub struct HeaderInjector<'a>(pub &'a mut http::HeaderMap);

#[cfg(feature = "tracing")]

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    /// Set a key and value in the HeaderMap.  Does nothing if the key or value are not valid inputs.
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = http::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = http::header::HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

/// Helper for extracting headers from HTTP Requests. This is used for OpenTelemetry context
/// propagation over HTTP.
/// See [this](https://github.com/open-telemetry/opentelemetry-rust/blob/main/examples/tracing-http-propagator/README.md)
/// for example usage.
#[cfg(feature = "tracing")]
pub struct HeaderExtractor<'a>(pub &'a http::HeaderMap);

#[cfg(feature = "tracing")]
impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    /// Get a value for a key from the HeaderMap.  If the value is not valid ASCII, returns None.
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    /// Collect all the keys from the HeaderMap.
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|value| value.as_str()).collect::<Vec<_>>()
    }
}
