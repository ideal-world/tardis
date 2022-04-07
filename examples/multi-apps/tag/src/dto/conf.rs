use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TagConfig {
    pub name_max_len: u8,
}

impl Default for TagConfig {
    fn default() -> Self {
        TagConfig { name_max_len: u8::MAX }
    }
}
