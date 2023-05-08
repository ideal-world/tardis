use tardis::basic::result::TardisResult;
use tardis::web::default_empty_mw::DefaultEmptyMW;
use tardis::web::web_server::TardisWebServer;
use tardis::TardisFuns;

use crate::api::tag_api::{self};
use crate::domain::tag;

pub async fn init(web_server: &TardisWebServer) -> TardisResult<()> {
    TardisFuns::reldb().conn().create_table_from_entity(tag::Entity).await?;
    web_server.add_module::<_, DefaultEmptyMW>("tag", tag_api::TagApi, None).await;
    Ok(())
}
