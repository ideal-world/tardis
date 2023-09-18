use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
pub struct MQModuleConfig {
    /// Message queue access Url, Url with permission information / 消息队列访问Url，Url带权限信息
    pub url: Url,
}
