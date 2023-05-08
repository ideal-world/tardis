use tardis::basic::result::TardisResult;
use tardis::web::default_empty_mw::DefaultEmptyMW;
use tardis::web::web_server::TardisWebServer;
use tardis::TardisFuns;

use crate::api::doc_api;
use crate::domain::doc;

pub async fn init(web_server: &TardisWebServer) -> TardisResult<()> {
    TardisFuns::reldb().conn().create_table_from_entity(doc::Entity).await?;
    web_server.add_module::<_, DefaultEmptyMW>("doc", doc_api::DocApi, None).await;
    Ok(())
}
