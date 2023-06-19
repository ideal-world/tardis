use tardis::basic::result::TardisResult;
use tardis::web::web_server::{TardisWebServer, EMPTY_MW};
use tardis::TardisFuns;

use crate::api::doc_api;
use crate::domain::doc;

pub async fn init(web_server: &TardisWebServer) -> TardisResult<()> {
    TardisFuns::reldb().conn().create_table_from_entity(doc::Entity).await?;
    web_server.add_module("doc", doc_api::DocApi, EMPTY_MW).await;
    Ok(())
}
