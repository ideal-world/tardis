use tardis::basic::dto::TardisContext;
use tardis::basic::error::TardisError;
use tardis::basic::result::TardisResult;
use tardis::db::sea_orm::*;
use tardis::db::sea_query::{Expr, Query};
use tardis::TardisFunsInst;

use crate::domain::tag;
use crate::dto::conf::TagConfig;
use crate::dto::tag_dto::{TagAddReq, TagResp};

pub struct TagServ;

impl<'a> TagServ {
    pub async fn add_doc(add_req: &TagAddReq, funs: &TardisFunsInst<'a>, cxt: &TardisContext) -> TardisResult<TagResp> {
        if funs.conf::<TagConfig>().name_max_len < add_req.name.len() as u8 {
            return Err(TardisError::BadRequest("name too long".to_string()));
        }
        let doc = tag::ActiveModel {
            name: Set(add_req.name.to_string()),
            create_id: Set(add_req.create_id.to_string()),
            ..Default::default()
        };
        let result = funs.db().insert_one(doc, cxt).await?;

        let resp = funs
            .db()
            .get_dto(
                Query::select()
                    .column(tag::Column::Id)
                    .column(tag::Column::Name)
                    .column(tag::Column::CreateId)
                    .from(tag::Entity)
                    .and_where(Expr::col(tag::Column::Id).eq(result.last_insert_id)),
            )
            .await?
            .unwrap();
        Ok(resp)
    }
}
