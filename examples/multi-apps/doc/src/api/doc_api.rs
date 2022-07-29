use tardis::web::context_extractor::TardisContextExtractor;
use tardis::web::poem_openapi;
use tardis::web::poem_openapi::payload::Json;
use tardis::web::web_resp::{TardisApiResult, TardisResp};
use tardis::TardisFuns;

use crate::dto::doc_dto::DocAddReq;
use crate::serv::doc_serv::DocServ;

pub struct DocApi;

#[poem_openapi::OpenApi(prefix_path = "/doc")]
impl DocApi {
    /// Add
    #[oai(path = "/", method = "post")]
    async fn add(&self, add_req: Json<DocAddReq>, ctx: TardisContextExtractor) -> TardisApiResult<i32> {
        let mut funs = TardisFuns::inst_with_db_conn("doc".to_string(), None);
        funs.begin().await?;
        let result = DocServ::add_doc(&add_req.0, &funs, &ctx.0).await?;
        funs.commit().await?;
        TardisResp::ok(result)
    }
}
