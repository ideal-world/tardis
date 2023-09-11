use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
pub struct MQModuleConfig {
    #[builder(default)]
    /// Message queue access Url, Url with permission information / 消息队列访问Url，Url带权限信息
    pub url: String,
}