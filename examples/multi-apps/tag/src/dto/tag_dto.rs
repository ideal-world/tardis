use std::fmt::Debug;

use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

use tardis::web::poem_openapi::Object;

#[derive(Object, Serialize, Deserialize, Debug)]
#[oai(rename_all = "camelCase")]
pub struct TagAddReq {
    pub name: String,
    pub create_id: String,
}

#[derive(Object, Serialize, Deserialize, Debug, FromQueryResult)]
#[oai(rename_all = "camelCase")]
pub struct TagResp {
    pub id: i32,
    pub name: String,
    pub create_id: String,
}
