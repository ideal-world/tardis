use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(default)]
pub struct OSModuleConfig {
    /// s3/oss/obs, Support amazon s3 / aliyun oss / huaweicloud obs
    #[builder(default = "s3".to_string())]
    pub kind: String,
    #[builder(default)]
    pub endpoint: String,
    #[builder(default)]
    pub ak: String,
    #[builder(default)]
    pub sk: String,
    #[builder(default)]
    pub region: String,
    #[builder(default)]
    pub default_bucket: String,
}

impl Default for OSModuleConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}