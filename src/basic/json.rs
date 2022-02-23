use crate::serde::de::DeserializeOwned;
use crate::serde::{Deserialize, Serialize};
use crate::serde_json::Value;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

pub struct TardisJson;

impl TardisJson {
    pub fn str_to_obj<'a, T: Deserialize<'a>>(&self, str: &'a str) -> TardisResult<T> {
        let result = serde_json::from_str::<'a, T>(str);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    pub fn str_to_json<'a>(&self, str: &'a str) -> TardisResult<Value> {
        let result = serde_json::from_str::<'a, Value>(str);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    pub fn json_to_obj<T: DeserializeOwned>(&self, value: Value) -> TardisResult<T> {
        let result = serde_json::from_value::<T>(value);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    pub fn obj_to_string<T: ?Sized + Serialize>(&self, obj: &T) -> TardisResult<String> {
        let result = serde_json::to_string(obj);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    pub fn obj_to_json<T: Serialize>(&self, obj: &T) -> TardisResult<Value> {
        let result = serde_json::to_value(obj);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    pub fn json_to_string(&self, value: Value) -> TardisResult<String> {
        let result = serde_json::to_string(&value);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }
}
