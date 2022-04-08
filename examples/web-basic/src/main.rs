use std::env;

use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::TardisFuns;

use crate::processor::Api;

mod processor;

///
/// Visit: http://127.0.0.1:8089/ui
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");
    // Initial configuration
    TardisFuns::init("config").await?;
    // Register the processor and start the web service
    TardisFuns::web_server().add_route(Api).await.start().await
}
