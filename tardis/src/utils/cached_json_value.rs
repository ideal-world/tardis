use std::{
    any::Any,
    sync::{Arc, OnceLock},
};

use serde::Deserialize;

pub struct CachedJsonValue {
    json_value: serde_json::Value,
    cache: OnceLock<Arc<dyn Any + 'static + Send + Sync>>,
}

impl CachedJsonValue {
    pub fn new(json_value: serde_json::Value) -> Self {
        Self {
            json_value,
            cache: OnceLock::new(),
        }
    }
    pub fn get<T: for<'a> Deserialize<'a> + Any + 'static + Send + Sync>(&self) -> serde_json::Result<Arc<T>> {
        if let Some(v) = self.cache.get() {
            return Ok(v.clone().downcast::<T>().expect("invalid type downcasted"));
        }
        let rust_value: Arc<T> = Arc::new(serde_json::from_value(self.json_value.clone())?);
        let cached = rust_value.clone();
        self.cache.get_or_init(|| cached);
        Ok(rust_value)
    }
}
