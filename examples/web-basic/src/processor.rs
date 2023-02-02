use tardis::basic::error::TardisError;
use tardis::web::poem_openapi;
use tardis::web::poem_openapi::param::Query;
use tardis::web::web_resp::{TardisApiResult, TardisResp};

pub struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> TardisApiResult<String> {
        match name.0 {
            Some(name) => TardisResp::ok(format!("hello, {name}!")),
            None => TardisResp::err(TardisError::not_found("name does not exist", "")),
        }
    }
}
