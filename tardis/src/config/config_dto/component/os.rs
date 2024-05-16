use serde::{Deserialize, Serialize};

use typed_builder::TypedBuilder;

use crate::redact::Redact;

#[derive(Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct OSModuleConfig {
    /// s3/oss/obs, Support amazon s3 / aliyun oss / huaweicloud obs
    #[builder(default = "s3".to_string(), setter(into))]
    pub kind: String,
    #[builder(default, setter(into))]
    pub endpoint: String,
    #[builder(default, setter(into))]
    pub ak: String,
    #[builder(default, setter(into))]
    pub sk: String,
    #[builder(default, setter(into))]
    pub region: String,
    #[builder(default, setter(into))]
    pub default_bucket: String,
}

impl std::fmt::Debug for OSModuleConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OSModuleConfig")
            .field("kind", &self.kind)
            .field("endpoint", &self.endpoint)
            .field("ak", &self.ak)
            .field("sk", &self.sk.redact())
            .field("region", &self.region)
            .field("default_bucket", &self.default_bucket)
            .finish()
    }
}

impl Default for OSModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}
