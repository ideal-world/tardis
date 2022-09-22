use std::sync::atomic::{AtomicBool, Ordering};

use crate::basic::result::TardisResult;

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
        tracing_subscriber::fmt::init();
        Ok(())
    }
}
