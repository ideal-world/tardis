use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::serde::de::Deserialize;
use crate::serde::Serialize;
use crate::serde_json::Value;
use std::fs::File;
use std::path::Path;

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
    ///
    /// # Errors
    /// input is not a valid json string
    pub fn str_to_obj<T: for<'de> Deserialize<'de>>(&self, str: &str) -> TardisResult<T> {
        let result = serde_json::from_str(str);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-str-to-obj-error")),
        }
    }

    /// Convert [`std::io::Read`] trait to Rust object / 将Read trait转换为Rust对象 \
    /// see [`serde_json::from_reader`]
    /// # Arguments
    ///
    /// * `rdr` - impl [`std::io::Read`]trait/ impl Read trait 对象
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// use tardis::serde_json::Value;
    ///
    /// let file = fs::File::open("text.json")?
    /// TardisFuns::json.reader_to_obj::<Value>(file);
    /// ```
    /// 
    /// # Errors
    /// see [`serde_json::from_reader`]
    pub fn reader_to_obj<R: std::io::Read, T: for<'de> Deserialize<'de>>(&self, rdr: R) -> TardisResult<T> {
        let result = serde_json::from_reader::<R, T>(rdr);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-reader-to-obj-error")),
        }
    }

    /// Read the contents of the file and convert it to a Rust object / 读取file文件内容转换为Rust对象
    /// # Arguments
    ///
    /// * `path` - file path/ file路径
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// use tardis::serde_json::Value;
    ///
    /// TardisFuns::json.file_to_obj::<Value, &str>("text.json")?;
    /// ```
    /// 
    /// # Errors
    /// 1. open file error, see [`std::fs::File::open`]
    /// 2. see [`serde_json::from_reader`]
    pub fn file_to_obj<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(&self, path: P) -> TardisResult<T> {
        let file = File::open(path);
        match file {
            Ok(f) => self.reader_to_obj::<File, T>(f),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-file-to-obj-error")),
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
    /// # Errors
    /// input is not a valid json string
    pub fn str_to_json(&self, str: &str) -> TardisResult<Value> {
        let result = serde_json::from_str(str);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-str-to-json-error")),
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
    pub fn json_to_obj<T: for<'de> Deserialize<'de>>(&self, value: Value) -> TardisResult<T> {
        let result = serde_json::from_value::<T>(value);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-json-to-obj-error")),
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
    /// # Errors
    /// see [`serde_json::to_string`]
    pub fn obj_to_string<T: ?Sized + Serialize>(&self, obj: &T) -> TardisResult<String> {
        let result = serde_json::to_string(obj);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-obj-to-str-error")),
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
    /// # Errors
    /// see [`serde_json::to_value`]
    pub fn obj_to_json<T: Serialize>(&self, obj: &T) -> TardisResult<Value> {
        let result = serde_json::to_value(obj);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-obj-to-json-error")),
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
    /// # Errors
    /// see [`serde_json::to_string`]
    /// 
    pub fn json_to_string(&self, value: Value) -> TardisResult<String> {
        let result = serde_json::to_string(&value);
        match result {
            Ok(r) => Ok(r),
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-json-to-str-error")),
        }
    }

    /// serialize into json value and deserialize into target type / 序列化为json值并反序列化为目标类型
    /// 
    /// # Errors
    /// 1. serialize error, see [`serde_json::to_value`]
    /// 2. deserialize error, see [`serde_json::from_value`]
    pub fn copy<F: Serialize, T: for<'de> Deserialize<'de>>(&self, source: &F) -> TardisResult<T> {
        let result = serde_json::to_value(source);
        match result {
            Ok(value) => {
                let result = serde_json::from_value::<T>(value);
                match result {
                    Ok(r) => Ok(r),
                    Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-copy-deserialize-error")),
                }
            }
            Err(error) => Err(TardisError::format_error(&format!("[Tardis.Json] {error:?}"), "406-tardis-json-copy-serialize-error")),
        }
    }
}
