use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DocConfig {
    pub content_max_len: u32,
}

impl Default for DocConfig {
    fn default() -> Self {
        DocConfig { content_max_len: u32::MAX }
    }
}
