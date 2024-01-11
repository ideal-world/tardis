use std::env;

use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::web::web_server::WebServerModule;
use tardis::TardisFuns;
use tardis::web::ws_processor::TardisWebsocketMgrMessage;

use crate::processor::Page;

mod processor;

///
/// Visit: https://127.0.0.1:8089/echo
/// Visit: https://127.0.0.1:8089/broadcast
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    env::set_var("PROFILE", "default");
    // Initial configuration
    TardisFuns::init(Some("config")).await?;

    TardisFuns::web_server().add_route(WebServerModule::from(Page).with_ws::<TardisWebsocketMgrMessage>(100)).await.start().await?;
    TardisFuns::web_server().await;
    Ok(())
}
