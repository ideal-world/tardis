//! Common DTOs / 常用的DTO
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::serde::{Deserialize, Serialize};

use super::result::TardisResult;

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
    /// List of requested role ids / 请求的角色Id列表
    pub roles: Vec<String>,
    /// List of requested group ids / 请求的群组Id列表
    pub groups: Vec<String>,
    /// Extension information / 扩展信息
    #[serde(skip)]
    pub ext: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for TardisContext {
    fn default() -> Self {
        TardisContext {
            own_paths: "".to_string(),
            ak: "".to_string(),
            owner: "".to_string(),
            roles: vec![],
            groups: vec![],
            ext: Default::default(),
        }
    }
}

impl TardisContext {
    pub fn add_ext(&self, key: &str, value: &str) -> TardisResult<()> {
        self.ext.write()?.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn remove_ext(&self, key: &str) -> TardisResult<()> {
        self.ext.write()?.remove(key);
        Ok(())
    }

    pub fn get_ext(&self, key: &str) -> TardisResult<Option<String>> {
        Ok(self.ext.read()?.get(key).cloned())
    }
}
