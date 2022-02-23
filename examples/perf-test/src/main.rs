use std::env;

use tardis::basic::config::NoneConfig;
use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::TardisFuns;

use crate::processor::TodoApi;

mod domain;
mod initializer;
mod processor;

///
/// Visit: http://127.0.0.1:8089/ui
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info");
    // Initial
    TardisFuns::init::<NoneConfig>("config").await?;
    initializer::init().await?;
    // Register the processor and start the web service
    TardisFuns::web_server().add_module("", TodoApi).start().await
}
