use tardis::basic::dto::TardisFunsInst;
use tardis::web::context_extractor::TardisContextExtractor;
use tardis::web::poem_openapi::{payload::Json, OpenApi};
use tardis::web::web_resp::{TardisApiResult, TardisResp};

use crate::dto::tag_dto::TagAddReq;
use crate::serv::tag_serv::TagServ;

pub struct TagApi;

#[OpenApi(prefix_path = "/tag")]
impl TagApi {
    /// Add
    #[oai(path = "/", method = "post")]
    async fn add(&self, add_req: Json<TagAddReq>, cxt: TardisContextExtractor) -> TardisApiResult<i32> {
        let mut funs = TardisFunsInst::conn("tag");
        funs.begin().await?;
        let result = TagServ::add_doc(&add_req.0, &funs, &cxt.0).await?;
        funs.commit().await?;
        TardisResp::ok(result)
    }
}
