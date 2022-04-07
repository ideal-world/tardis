use tardis::basic::dto::{TardisContext, TardisFunsInst};
use tardis::basic::error::TardisError;
use tardis::basic::result::TardisResult;
use tardis::db::sea_orm::*;

use crate::domain::tag;
use crate::dto::conf::TagConfig;
use crate::dto::tag_dto::TagAddReq;

pub struct TagServ;

impl<'a> TagServ {
    pub async fn add_doc(add_req: &TagAddReq, funs: &TardisFunsInst<'a>, cxt: &TardisContext) -> TardisResult<i32> {
        if funs.conf::<TagConfig>().name_max_len < add_req.name.len() as u8 {
            return Err(TardisError::BadRequest("name too long".to_string()));
        }
        let doc = tag::ActiveModel {
            name: Set(add_req.name.to_string()),
            ..Default::default()
        };
        let result = funs.db().insert_one(doc, cxt).await?;
        Ok(result.last_insert_id)
    }
}
