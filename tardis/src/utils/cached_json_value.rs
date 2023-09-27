use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use serde::Deserialize;

pub struct CachedJsonValue {
    json_value: serde_json::Value,
    cache: OnceLock<RwLock<HashMap<TypeId, Arc<dyn Any + 'static + Send + Sync>>>>,
}

impl CachedJsonValue {
    pub fn new(json_value: serde_json::Value) -> Self {
        Self {
            json_value,
            cache: OnceLock::new(),
        }
    }
    pub fn get<T: for<'a> Deserialize<'a> + Any + 'static + Send + Sync>(&self) -> serde_json::Result<Arc<T>> {
        let lock = self.cache.get_or_init(Default::default);
        {
            let rg = lock.read().expect("poisoned map");
            if let Some(v) = rg.get(&TypeId::of::<T>()) {
                return Ok(v.clone().downcast::<T>().expect("invalid type downcasted"));
            }
        }
        {
            let rust_value: Arc<T> = Arc::new(serde_json::from_value(self.json_value.clone())?);
            let mut wg = lock.write().expect("poisoned map");
            wg.insert(TypeId::of::<T>(), rust_value.clone());
            Ok(rust_value)
        }
    }
    pub fn raw(&self) -> &serde_json::Value {
        &self.json_value
    }
}
