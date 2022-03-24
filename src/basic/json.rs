use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::serde::de::DeserializeOwned;
use crate::serde::{Deserialize, Serialize};
use crate::serde_json::Value;

/// Json handle / Json处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let test_config = TestConfig {
///     project_name: "测试".to_string(),
///     level_num: 0,
///     db_proj: DatabaseConfig { url: "http://xxx".to_string() },
/// };
///
/// let json_str = TardisFuns::json.obj_to_string(&test_config)?;
/// assert_eq!(json_str, r#"{"project_name":"测试","level_num":0,"db_proj":{"url":"http://xxx"}}"#);
///
/// let json_obj = TardisFuns::json.str_to_obj::<TestConfig<DatabaseConfig>>(&json_str)?;
/// assert_eq!(json_obj.project_name, "测试");
/// assert_eq!(json_obj.level_num, 0);
/// assert_eq!(json_obj.db_proj.url, "http://xxx");
///
/// let json_value = TardisFuns::json.str_to_json(&json_str)?;
/// assert_eq!(json_value["project_name"], "测试");
/// assert_eq!(json_value["level_num"], 0);
/// assert_eq!(json_value["db_proj"]["url"], "http://xxx");
///
/// let json_value = TardisFuns::json.obj_to_json(&json_obj)?;
/// assert_eq!(json_value["project_name"], "测试");
/// assert_eq!(json_value["level_num"], 0);
/// assert_eq!(json_value["db_proj"]["url"], "http://xxx");
///
/// let json_obj = TardisFuns::json.json_to_obj::<TestConfig<DatabaseConfig>>(json_value)?;
/// assert_eq!(json_obj.project_name, "测试");
/// assert_eq!(json_obj.level_num, 0);
/// assert_eq!(json_obj.db_proj.url, "http://xxx");
/// ```
pub struct TardisJson;

impl TardisJson {
    /// Convert Json string to Rust object / 将Json字符串转换为Rust对象
    ///
    /// # Arguments
    ///
    /// * `str` - Json string / Json字符串
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.str_to_obj::<TestConfig<DatabaseConfig>>(&json_str);
    /// ```
    pub fn str_to_obj<'a, T: Deserialize<'a>>(&self, str: &'a str) -> TardisResult<T> {
        let result = serde_json::from_str::<'a, T>(str);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    /// Convert Json string to Json object / 将Json字符串转换为Json对象
    ///
    /// # Arguments
    ///
    /// * `str` - Json string / Json字符串
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.str_to_json(&json_str);
    /// ```
    pub fn str_to_json<'a>(&self, str: &'a str) -> TardisResult<Value> {
        let result = serde_json::from_str::<'a, Value>(str);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    /// Convert Json object to Rust object / 将Json对象转换为Rust对象
    ///
    /// # Arguments
    ///
    /// * `value` - Json object / Json对象
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.json_to_obj::<TestConfig<DatabaseConfig>>(json_value);
    /// ```
    pub fn json_to_obj<T: DeserializeOwned>(&self, value: Value) -> TardisResult<T> {
        let result = serde_json::from_value::<T>(value);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    /// Convert Rust string to Json string / 将Rust对象转换为Json字符串
    ///
    /// # Arguments
    ///
    /// * `obj` - Rust object  / Rust对象
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.obj_to_string(&rust_obj);
    /// ```
    pub fn obj_to_string<T: ?Sized + Serialize>(&self, obj: &T) -> TardisResult<String> {
        let result = serde_json::to_string(obj);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    /// Convert Rust object to Json object / 将Rust对象转换为Json对象
    ///
    /// # Arguments
    ///
    /// * `obj` - Rust object  / Rust对象
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.obj_to_json(&rust_obj);
    /// ```
    pub fn obj_to_json<T: Serialize>(&self, obj: &T) -> TardisResult<Value> {
        let result = serde_json::to_value(obj);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }

    /// Convert Json object to Json string / 将Json对象转换成Json字符串
    ///
    /// # Arguments
    ///
    /// * `value` - Json object / Json对象
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::json.json_to_string(json_value);
    /// ```
    pub fn json_to_string(&self, value: Value) -> TardisResult<String> {
        let result = serde_json::to_string(&value);
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(TardisError::Box(Box::new(e))),
        }
    }
}
