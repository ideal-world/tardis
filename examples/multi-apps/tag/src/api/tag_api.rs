use crate::dto::tag_dto::{TagAddReq, TagResp};
use crate::serv::tag_serv::TagServ;
use tardis::web::context_extractor::TardisContextExtractor;
use tardis::web::poem_openapi;
use tardis::web::poem_openapi::payload::Json;
use tardis::web::web_resp::{TardisApiResult, TardisResp};
use tardis::TardisFuns;

pub struct TagApi;

#[poem_openapi::OpenApi(prefix_path = "/tag")]
impl TagApi {
    /// Add
    #[oai(path = "/", method = "post")]
    async fn add(&self, add_req: Json<TagAddReq>, ctx: TardisContextExtractor) -> TardisApiResult<TagResp> {
        let mut funs = TardisFuns::inst_with_db_conn("tag".to_string(), None);
        funs.begin().await?;
        let result = TagServ::add_doc(&add_req.0, &funs, &ctx.0).await?;
        funs.commit().await?;
        TardisResp::ok(result)
    }
}
