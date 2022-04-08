use std::env;

use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::web::poem::{get, EndpointExt, Route};
use tardis::TardisFuns;

use crate::processor::ws_broadcast;
use crate::processor::ws_p2p;
use crate::processor::Page;

mod processor;

///
/// Visit: http://127.0.0.1:8089/p2p
/// Visit: http://127.0.0.1:8089/broadcast
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");
    // Initial configuration
    TardisFuns::init("config").await?;

    let mut ws_route = Route::new();
    ws_route = ws_route.at("/broadcast/:name", get(ws_broadcast.data(tokio::sync::broadcast::channel::<String>(32).0))).at("/p2p/:name", get(ws_p2p));
    TardisFuns::web_server().add_route(Page).await.add_module_raw("ws", ws_route).await.start().await
}
