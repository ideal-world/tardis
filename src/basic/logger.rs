use std::sync::Mutex;

use crate::basic::result::TardisResult;

lazy_static! {
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
}

pub struct TardisLogger;

impl TardisLogger {
    pub(crate) fn init() -> TardisResult<()> {
        let mut initialized = INITIALIZED.lock().unwrap();
        if *initialized == true {
            return Ok(());
        }
        *initialized = true;

        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "info");
        }

        tracing_subscriber::fmt::init();
        Ok(())
    }
}
