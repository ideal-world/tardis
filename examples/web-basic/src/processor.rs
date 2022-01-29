use poem_openapi::param::Query;
use poem_openapi::OpenApi;

use tardis::basic::error::TardisError;
use tardis::web::web_resp::TardisResp;

pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> TardisResp<String> {
        match name.0 {
            Some(name) => TardisResp::ok(format!("hello, {}!", name)),
            None => TardisResp::err(TardisError::NotFound("name does not exist".to_string())),
        }
    }
}
