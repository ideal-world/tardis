use tardis::basic::result::TardisResult;
use tardis::web::web_server::TardisWebServerInner;
use tardis::TardisFuns;

use crate::api::doc_api;
use crate::domain::doc;

pub async fn init(web_server: &TardisWebServerInner) -> TardisResult<()> {
    TardisFuns::reldb().conn().create_table_from_entity(doc::Entity).await?;
    web_server.add_module("doc", doc_api::DocApi).await;
    Ok(())
}
