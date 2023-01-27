use std::env;

use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::TardisFuns;

use crate::processor::Page;

mod processor;

///
/// Visit: http://127.0.0.1:8089/echo
/// Visit: http://127.0.0.1:8089/broadcast
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    env::set_var("PROFILE", "default");
    // Initial configuration
    TardisFuns::init("config").await?;

    TardisFuns::web_server().add_route_with_ws(Page, 100).await.start().await
}
