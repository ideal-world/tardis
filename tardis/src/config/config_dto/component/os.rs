use serde::{Deserialize, Serialize};

use typed_builder::TypedBuilder;

#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
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

impl Default for OSModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}
