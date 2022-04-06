//! Common DTOs / 常用的DTO
use serde::de::DeserializeOwned;

use crate::serde::{Deserialize, Serialize};
use crate::{TardisFuns, TardisResult};

/// ardis context / Tardis上下文
///
/// Used to bring in some authentication information when a web request is received.
///
/// 用于Web请求时带入一些认证信息.
///
/// This information needs to be supported by the IAM service.
///
/// 该信息需要与 IAM 服务对应.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct TardisContext {
    /// The requested scope ids / 请求的作用域Ids
    pub scope_paths: String,
    /// The requested Ak / 请求的Ak
    pub ak: String,
    /// The requested account id / 请求的账号Id
    pub account_id: String,
    /// The requested Token / 请求的Token
    pub token: String,
    /// The requested Token type / 请求的Token类型
    pub token_kind: String,
    /// List of requested role ids / 请求的角色Id列表
    pub roles: Vec<String>,
    /// List of requested group ids / 请求的群组Id列表
    pub groups: Vec<String>,
}

impl Default for TardisContext {
    fn default() -> Self {
        TardisContext {
            scope_paths: "".to_string(),
            ak: "".to_string(),
            account_id: "".to_string(),
            token: "".to_string(),
            token_kind: "".to_string(),
            roles: vec![],
            groups: vec![],
        }
    }
}

pub struct TardisFunsInst<'a> {
    module_code: &'a str,
    #[cfg(feature = "reldb")]
    db: Option<crate::db::reldb_client::TardisRelDBlConnection<'a>>,
}

impl<'a> TardisFunsInst<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            module_code: code,
            #[cfg(feature = "reldb")]
            db: None,
        }
    }

    pub fn conf<T: 'static + DeserializeOwned>(code: &'a str) -> &T {
        TardisFuns::cs_config(code)
    }

    #[cfg(feature = "reldb")]
    pub fn conn(code: &'a str) -> Self {
        let reldb = TardisFuns::reldb_by_module_or_default(code);
        Self {
            module_code: code,
            db: Some(reldb.conn()),
        }
    }

    #[cfg(feature = "reldb")]
    pub fn reldb(&self) -> &'static crate::TardisRelDBClient {
        TardisFuns::reldb_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "reldb")]
    pub fn db(&self) -> &crate::db::reldb_client::TardisRelDBlConnection<'a> {
        self.db.as_ref().expect("db is not initialized")
    }

    #[cfg(feature = "reldb")]
    pub async fn begin(&mut self) -> TardisResult<()> {
        self.db.as_mut().expect("db is not initialized").begin().await
    }

    #[cfg(feature = "reldb")]
    pub async fn commit(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").commit().await
    }

    #[cfg(feature = "reldb")]
    pub async fn rollback(self) -> TardisResult<()> {
        self.db.expect("db is not initialized").rollback().await
    }

    #[cfg(feature = "cache")]
    pub fn cache(&self) -> &'static crate::TardisCacheClient {
        TardisFuns::cache_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "mq")]
    pub fn mq(&self) -> &'static crate::TardisMQClient {
        TardisFuns::mq_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn web_client(&self) -> &'static crate::TardisWebClient {
        TardisFuns::web_client_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn search(&self) -> &'static crate::TardisSearchClient {
        TardisFuns::search_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "mail")]
    pub fn mail(&self) -> &'static crate::TardisMailClient {
        TardisFuns::mail_by_module_or_default(self.module_code)
    }

    #[cfg(feature = "os")]
    pub fn os(&self) -> &'static crate::TardisOSClient {
        TardisFuns::os_by_module_or_default(self.module_code)
    }
}
