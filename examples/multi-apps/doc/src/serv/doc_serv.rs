use tardis::basic::dto::TardisContext;
use tardis::basic::error::TardisError;
use tardis::basic::result::TardisResult;
use tardis::db::sea_orm::*;
use tardis::TardisFunsInst;

use crate::domain::doc;
use crate::dto::conf::DocConfig;
use crate::dto::doc_dto::DocAddReq;

pub struct DocServ;

impl DocServ {
    pub async fn add_doc(add_req: &DocAddReq, funs: &TardisFunsInst, ctx: &TardisContext) -> TardisResult<i32> {
        if funs.conf::<DocConfig>().content_max_len < add_req.content.len() as u32 {
            return Err(TardisError::bad_request("content too long", ""));
        }
        let doc = doc::ActiveModel {
            name: Set(add_req.name.to_string()),
            content: Set(add_req.content.to_string()),
            ..Default::default()
        };
        let result = funs.db().insert_one(doc, ctx).await?;
        Ok(result.last_insert_id)
    }
}
