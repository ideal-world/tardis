use poem::Request;
use poem_openapi::{auth::ApiKey, SecurityScheme};

use crate::basic::dto::TardisContext;
use crate::basic::error::TardisError;
use crate::{TardisFuns, TardisResult};

pub const TOKEN_FLAG: &str = "__";

#[derive(SecurityScheme)]
#[oai(type = "api_key", key_name = "Tardis-Context", in = "header", checker = "context_checker")]
pub struct TardisContextExtractor(pub TardisContext);

pub trait TardisContextExtractFun {
    fn extract_context(&self, req: &Request) -> TardisResult<TardisContext>;
}

async fn context_checker(req: &Request, _: ApiKey) -> Option<TardisContext> {
    match extract_context(req).await {
        Ok(context) => Some(context),
        Err(err) => {
            log::warn!("{} at {}", err.message(), req.uri());
            None
        }
    }
}

async fn extract_context(req: &Request) -> TardisResult<TardisContext> {
    let context_header_name = &TardisFuns::fw_config().web_server.context_conf.context_header_name;
    let context = req
        .headers()
        .get(context_header_name)
        .ok_or_else(|| TardisError::BadRequest(format!("[Tardis.WebServer] {} is not found", context_header_name)))?
        .to_str()
        .map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not string".to_string()))?;
    if !context.starts_with(TOKEN_FLAG) {
        let context = base64::decode(context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not base64".to_string()))?;
        let context = String::from_utf8(context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not utf8".to_string()))?;
        let context = TardisFuns::json.str_to_obj(&context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context header is not valid json".to_string()))?;
        Ok(context)
    } else {
        #[cfg(feature = "cache")]
        {
            let token = context.split(TOKEN_FLAG).nth(1).ok_or_else(|| TardisError::BadRequest("[Tardis.WebServer] Context header is not valid".to_string()))?;
            let context = TardisFuns::cache().get(format!("{}{}", TardisFuns::fw_config().web_server.context_conf.token_cache_key, token).as_str()).await?;
            let context = context.ok_or_else(|| TardisError::BadRequest("[Tardis.WebServer] Token is not in cache".to_string()))?;
            let context = TardisFuns::json.str_to_obj(&context).map_err(|_| TardisError::BadRequest("[Tardis.WebServer] Context cache is not valid json".to_string()))?;
            Ok(context)
        }
        #[cfg(not(feature = "cache"))]
        {
            Err(TardisError::BadRequest("[Tardis.WebServer] Context is not found".to_string()))
        }
    }
}
