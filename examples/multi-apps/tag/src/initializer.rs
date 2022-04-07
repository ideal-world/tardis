use tardis::basic::result::TardisResult;
use tardis::web::web_server::TardisWebServer;
use tardis::TardisFuns;

use crate::api::tag_api;
use crate::domain::tag;

pub async fn init(web_server: &TardisWebServer) -> TardisResult<()> {
    TardisFuns::reldb().conn().create_table_from_entity(tag::Entity).await?;
    web_server.add_module("tag", tag_api::TagApi).await;
    Ok(())
}
