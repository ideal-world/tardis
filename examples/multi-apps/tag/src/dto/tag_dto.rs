use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use tardis::db::sea_orm;
use tardis::web::poem_openapi;

#[derive(poem_openapi::Object, Serialize, Deserialize, Debug)]
#[oai(rename_all = "camelCase")]
pub struct TagAddReq {
    pub name: String,
    pub create_id: String,
}

#[derive(poem_openapi::Object, Serialize, Deserialize, Debug, sea_orm::FromQueryResult)]
#[oai(rename_all = "camelCase")]
pub struct TagResp {
    pub id: i32,
    pub name: String,
    pub create_id: String,
}
