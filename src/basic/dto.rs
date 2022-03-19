//! Common DTOs / 常用的DTO
use crate::serde::{Deserialize, Serialize};

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
    /// The requested application Code / 请求的应用编码
    pub app_code: String,
    ///  The requested tenant Code / 请求的租户编码
    pub tenant_code: String,
    /// The requested Ak / 请求的Ak
    pub ak: String,
    /// The requested account code / 请求的账号编码
    pub account_code: String,
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
            app_code: "".to_string(),
            tenant_code: "".to_string(),
            ak: "".to_string(),
            account_code: "".to_string(),
            token: "".to_string(),
            token_kind: "".to_string(),
            roles: vec![],
            groups: vec![],
        }
    }
}
