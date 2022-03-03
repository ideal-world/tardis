use async_trait::async_trait;
use poem::Request;

use crate::basic::dto::TardisContext;
use crate::basic::error::TardisError;
use crate::{TardisFuns, TardisResult};

#[async_trait]
pub trait ContextExtractor {
    async fn extract_context(&self) -> TardisResult<TardisContext>;
}

#[async_trait]
impl ContextExtractor for Request {
    async fn extract_context(&self) -> TardisResult<TardisContext> {
        if let Some(context_header_name) = &TardisFuns::fw_config().web_server.context_conf.context_header_name {
            if let Some(context) = self.headers().get(context_header_name) {
                let context = context.to_str().map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not string".to_string()))?;
                let context = base64::decode(context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer]Context header is not base64".to_string()))?;
                let context = String::from_utf8(context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not utf8".to_string()))?;
                let context = TardisFuns::json.str_to_obj(&context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not valid json".to_string()))?;
                return Ok(context);
            }
        }
        #[cfg(feature = "cache")]
        if let Some(token_header_name) = &TardisFuns::fw_config().web_server.context_conf.token_header_name {
            if let Some(token) = self.headers().get(token_header_name) {
                let token = token.to_str().map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Token header is not string".to_string()))?;
                let context = TardisFuns::cache().get(format!("{}{}", TardisFuns::fw_config().web_server.context_conf.token_redis_key, token).as_str()).await?;
                if let Some(context) = context {
                    let context = TardisFuns::json.str_to_obj(&context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context cache is not valid json".to_string()))?;
                    return Ok(context);
                } else {
                    return Err(TardisError::BadRequest("[Tardis.WebServer] Token is not in cache".to_string()));
                }
            }
        }
        Err(TardisError::BadRequest("[Tardis.WebServer] Context is not found".to_string()))
    }
}
