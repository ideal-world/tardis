use std::env;

use testcontainers::clients;

use tardis::basic::config::NoneConfig;
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
    let docker = clients::Cli::default();
    let mysql_container = TardisTestContainer::mysql_custom(None, &docker);
    let port = mysql_container.get_host_port(3306).expect("Test port acquisition error");
    let url = format!("mysql://root:123456@localhost:{}/test", port);
    env::set_var("TARDIS_DB.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");
    // Initial
    TardisFuns::init::<NoneConfig>("config").await?;
    initializer::init().await?;
    // Register the processor and start the web service
    TardisFuns::web_server().add_module("", TodoApi).await.start().await
}
