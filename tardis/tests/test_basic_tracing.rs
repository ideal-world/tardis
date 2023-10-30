use crate::app::req::test_req;
use tardis::basic::{result::TardisResult, tracing::TardisTracing};
use tracing::{error, info};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_basic_tracing() -> TardisResult<()> {
    // env::set_var("RUST_LOG", "OFF");
    // env::set_var("RUST_LOG", "info");
    let _tardis_tracing = TardisTracing::initializer().with_fmt_layer().with_env_layer().with_opentelemetry_layer().init_standalone();
    // TardisFuns::init(Some("tests/config")).await?;
    let _g = tracing::trace_span!("main");
    let _g = _g.enter();
    info!("main info...");
    error!("main error");
    test_req().await;
    drop(_g);
    Ok(())
}

mod app {
    pub mod req {
        use tardis::log::{error, info};
        use tracing::instrument;
        #[instrument(level = "info", fields(module = env!("CARGO_PKG_NAME")))]
        pub async fn test_req() {
            info!("app::req info");
            error!("app::req error");
        }
    }
}
