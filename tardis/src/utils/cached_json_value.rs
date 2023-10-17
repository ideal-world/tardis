use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use serde::Deserialize;

/// # Cached Json Value
/// A wrapper of `serde_json::Value` with cache. The cache stores the deserialized value of the json value in different types.
///
/// # Example
/// ```ignore
/// #[derive(Deserialize)]
/// struct A {
///    a: i32,
/// }
/// #[derive(Deserialize)]
/// struct B {
///    b: String,
/// }
/// #[derive(Deserialize)]
/// struct C {
///    c: bool,
/// }
///
/// let json_value = serde_json::json!({
///     "a": 1,
///     "b": "2",
///     "c": true,
/// });
/// let cached_json_value = CachedJsonValue::new(json_value);
/// let a = cached_json_value.get::<A>().unwrap();
/// let b = cached_json_value.get::<B>().unwrap();
/// let c = cached_json_value.get::<C>().unwrap();
/// let raw_json_value = cached_json_value.raw();
/// ```
pub struct CachedJsonValue {
    json_value: serde_json::Value,
    cache: OnceLock<RwLock<HashMap<TypeId, Arc<dyn Any + 'static + Send + Sync>>>>,
}

impl CachedJsonValue {
    /// Create a new `CachedJsonValue` from a `serde_json::Value`.
    pub fn new(json_value: serde_json::Value) -> Self {
        Self {
            json_value,
            cache: OnceLock::new(),
        }
    }

    /// Get the deserialized value from the cache. If the value is not in the cache, deserialize it and put it in the cache.
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
