use std::env;

use tardis::basic::result::TardisResult;
use tardis::log::{error, info};
use tardis::TardisFuns;

use crate::app::req::test_req;

#[tokio::test]
async fn test_basic_logger() -> TardisResult<()> {
    // env::set_var("RUST_LOG", "OFF");
    env::set_var("RUST_LOG", "error,test_basic_logger::app=info");
    TardisFuns::init_log()?;
    info!("main info...");
    error!("main error");
    test_req();
    Ok(())
}

mod app {
    pub mod req {
        use tardis::log::{error, info};

        pub fn test_req() {
            info!("app::req info");
            error!("app::req error");
        }
    }
}
