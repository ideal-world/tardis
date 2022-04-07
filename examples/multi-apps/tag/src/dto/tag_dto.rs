use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use tardis::web::poem_openapi::Object;

#[derive(Object, Serialize, Deserialize, Debug)]
pub struct TagAddReq {
    pub name: String,
}
