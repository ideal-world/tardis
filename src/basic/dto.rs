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
    /// The requested own paths / 请求的所属路径
    pub own_paths: String,
    /// The requested Ak / 请求的Ak
    pub ak: String,
    /// The requested owner/ 请求的所属者
    pub owner: String,
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
            own_paths: "".to_string(),
            ak: "".to_string(),
            owner: "".to_string(),
            token: "".to_string(),
            token_kind: "".to_string(),
            roles: vec![],
            groups: vec![],
        }
    }
}

pub struct TardisFunsInst<'a> {
    module_code: String,
    #[cfg(feature = "reldb")]
    db: Option<crate::db::reldb_client::TardisRelDBlConnection<'a>>,
    // Solve the 'a not used issue when the reldb feature is not enabled
    #[cfg(not(feature = "reldb"))]
    _t: Option<&'a str>,
}

impl<'a> TardisFunsInst<'a> {
    pub(crate) fn new(code: String) -> Self {
        Self {
            module_code: code.to_lowercase(),
            #[cfg(feature = "reldb")]
            db: None,
            #[cfg(not(feature = "reldb"))]
            _t: None,
        }
    }

    #[cfg(feature = "reldb")]
    pub(crate) fn new_with_db_conn(code: String) -> Self {
        let reldb = TardisFuns::reldb_by_module_or_default(&code);
        Self {
            module_code: code.to_lowercase(),
            db: Some(reldb.conn()),
        }
    }

    pub fn module_code(&self) -> &str {
        &self.module_code
    }

    pub fn conf<T: 'static + DeserializeOwned>(&self) -> &T {
        TardisFuns::cs_config(&self.module_code)
    }

    #[cfg(feature = "reldb")]
    pub fn reldb(&self) -> &'static crate::TardisRelDBClient {
        TardisFuns::reldb_by_module_or_default(&self.module_code)
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
        TardisFuns::cache_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "mq")]
    pub fn mq(&self) -> &'static crate::TardisMQClient {
        TardisFuns::mq_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn web_client(&self) -> &'static crate::TardisWebClient {
        TardisFuns::web_client_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "web-client")]
    pub fn search(&self) -> &'static crate::TardisSearchClient {
        TardisFuns::search_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "mail")]
    pub fn mail(&self) -> &'static crate::TardisMailClient {
        TardisFuns::mail_by_module_or_default(&self.module_code)
    }

    #[cfg(feature = "os")]
    pub fn os(&self) -> &'static crate::TardisOSClient {
        TardisFuns::os_by_module_or_default(&self.module_code)
    }
}
