use crate::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct TardisContext {
    pub app_id: String,
    pub tenant_id: String,
    pub ak: String,
    pub account_id: String,
    pub token: String,
    pub token_kind: String,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
}

impl Default for TardisContext {
    fn default() -> Self {
        TardisContext {
            app_id: "".to_string(),
            tenant_id: "".to_string(),
            ak: "".to_string(),
            account_id: "".to_string(),
            token: "".to_string(),
            token_kind: "".to_string(),
            roles: vec![],
            groups: vec![],
        }
    }
}
