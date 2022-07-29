use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use tardis::basic::field::TrimString;
use tardis::web::poem_openapi;

#[derive(poem_openapi::Object, Serialize, Deserialize, Debug)]
pub struct DocAddReq {
    #[oai(validator(min_length = "2", max_length = "255"))]
    pub name: TrimString,
    pub content: String,
}
