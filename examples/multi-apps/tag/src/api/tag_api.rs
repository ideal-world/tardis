use tardis::web::context_extractor::TardisContextExtractor;
use tardis::web::poem_openapi::{payload::Json, OpenApi};
use tardis::web::web_resp::{TardisApiResult, TardisResp};
use tardis::TardisFuns;

use crate::dto::tag_dto::{TagAddReq, TagResp};
use crate::serv::tag_serv::TagServ;

pub struct TagApi;

#[OpenApi(prefix_path = "/tag")]
impl TagApi {
    /// Add
    #[oai(path = "/", method = "post")]
    async fn add(&self, add_req: Json<TagAddReq>, cxt: TardisContextExtractor) -> TardisApiResult<TagResp> {
        let mut funs = TardisFuns::inst_with_db_conn("tag".to_string());
        funs.begin().await?;
        let result = TagServ::add_doc(&add_req.0, &funs, &cxt.0).await?;
        funs.commit().await?;
        TardisResp::ok(result)
    }
}
