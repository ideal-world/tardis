use std::env;

use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
use tardis::tokio;
use tardis::TardisFuns;

mod initializer;

///
/// ### Multi-application aggregation example
///
/// This example has two applications: tardis-example-multi-apps-doc / tardis-example-multi-apps-tag
///
/// Each application has its own configuration: DocConfig / TagConfig
///
/// Each application can also specify special features, such as web, database
///
/// ### Visit
///
/// http://127.0.0.1:8089/doc/ui / http://127.0.0.1:8089/tag/ui
///
/// Authentication: eyJvd25fcGF0aHMiOiAiIiwiYWsiOiAiIiwib3duZXIiOiAiIiwicm9sZXMiOiBbXSwiZ3JvdXBzIjogW119
///
#[tokio::main]
async fn main() -> TardisResult<()> {
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let mysql_container = TardisTestContainer::mysql_custom(None).await?;
    let port = mysql_container.get_host_port_ipv4(3306).await?;
    let url = format!("mysql://root:123456@localhost:{port}/test");
    env::set_var("TARDIS_FW.DB.URL", url.clone());
    env::set_var("TARDIS_FW.DB.MODULES.TAG.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    TardisFuns::init(Some("config")).await?;
    let web_server = TardisFuns::web_server();
    initializer::init(&web_server).await?;
    web_server.start().await?;
    web_server.await;
    Ok(())
}
