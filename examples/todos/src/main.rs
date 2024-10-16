use std::env;

use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
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
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let mysql_container = TardisTestContainer::mysql_custom(None).await?;
    let port = mysql_container.get_host_port_ipv4(3306).await?;
    let url = format!("mysql://root:123456@localhost:{port}/test");
    env::set_var("TARDIS_FW.DB.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");
    // Initial
    TardisFuns::init(Some("config")).await?;
    initializer::init().await?;
    // Register the processor and start the web service
    TardisFuns::web_server().add_route(TodoApi).await.start().await?;
    TardisFuns::web_server().await;
    Ok(())
}
