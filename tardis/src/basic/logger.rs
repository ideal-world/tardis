use std::{
    ffi::OsString,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::basic::result::TardisResult;
use std::sync::Mutex;
use tracing::metadata::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    filter, fmt,
    reload::{self, Handle},
    Registry,
};

use super::error::TardisError;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref RELOAD_HANDLE: Mutex<Option<Handle<LevelFilter, Registry>>> = Mutex::new(None);
}

pub struct TardisLogger;

impl TardisLogger {
    pub(crate) fn init() -> TardisResult<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        #[cfg(not(feature = "tracing"))]
        {
            let level = std::env::var_os("RUST_LOG").unwrap_or(OsString::from("info")).into_string().unwrap();
            let filter = filter::LevelFilter::from_str(level.as_str()).unwrap();
            let (filter, reload_handle) = reload::Layer::new(filter);
            tracing_subscriber::registry().with(filter).with(fmt::Layer::default()).init();
            let mut global_reload_handle = RELOAD_HANDLE.lock().map_err(|error| TardisError::internal_error(&format!("{error:?}"), ""))?;
            *global_reload_handle = Some(reload_handle);
        }
        Ok(())
    }
}
