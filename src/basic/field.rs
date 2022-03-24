use std::fmt::{Display, Formatter};

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

lazy_static! {
    static ref R_PHONE: Regex = Regex::new(r"^1(3\d|4[5-9]|5[0-35-9]|6[2567]|7[0-8]|8\d|9[0-35-9])\d{8}$").expect("Regular parsing error");
    static ref R_MAIL: Regex = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$")
        .expect("Regular parsing error");
    static ref R_CODE_NCS: Regex = Regex::new(r"^[a-z0-9_]+$").expect("Regular parsing error");
    static ref R_CODE_CS: Regex = Regex::new(r"^[A-Za-z0-9_]+$").expect("Regular parsing error");
}

static BASE62: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
static BASE36: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

pub static GENERAL_SPLIT: &str = "##";

/// Field handle / 字段处理
///
/// Provides some common regular, Id generation and other functions.
///
/// 提供了一些常用的正则判断、Id生成等功能.
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// assert!(TardisFuns::field.is_phone("18657120202"));
/// assert_eq!(TardisFuns::field.incr_by_base62("abcd1").unwrap(), "abcd2");
/// assert!(TardisFuns::field.incr_by_base62("999").is_none());
/// assert_eq!(TardisFuns::field.incr_by_base36("abcd1").unwrap(), "abcd2");
/// assert!(TardisFuns::field.incr_by_base36("999").is_none());
/// assert!(TardisFuns::field.is_code_cs("Adw834_dfds"));
/// assert!(!TardisFuns::field.is_code_cs(" Adw834_dfds"));
/// assert!(!TardisFuns::field.is_code_cs("Adw834_d-fds"));
/// assert!(TardisFuns::field.is_code_ncs("adon2_43323tr"));
/// assert!(!TardisFuns::field.is_code_ncs("adon2_43323tr "));
/// assert!(!TardisFuns::field.is_code_ncs("Adw834_dfds"));
/// assert_eq!(TardisFuns::field.nanoid().len(), 21);
/// assert_eq!(TardisFuns::field.nanoid_len(4).len(), 4);
/// ```
pub struct TardisField;

impl TardisField {
    /// Determine if it is a cell phone number (only supports mainland China) / 判断是否是手机号（仅支持中国大陆）
    pub fn is_phone(&self, phone: &str) -> bool {
        R_PHONE.is_match(phone)
    }

    /// Determine if it is a email / 判断是否是邮箱
    pub fn is_mail(&self, mail: &str) -> bool {
        R_MAIL.is_match(mail)
    }

    /// Determine if it contains only numbers, lowercase letters and underscores /
    /// 判断是否只包含数字、小写字母及下划线
    pub fn is_code_cs(&self, str: &str) -> bool {
        R_CODE_CS.is_match(str)
    }

    /// Determine if only numbers, upper and lower case letters and underscores are included /
    /// 判断是否只包含数字、大小写字母及下划线
    pub fn is_code_ncs(&self, str: &str) -> bool {
        R_CODE_NCS.is_match(str)
    }

    /// Generate NanoId / 生成NanoId
    pub fn nanoid(&self) -> String {
        nanoid::nanoid!()
    }

    /// Generate NanoId / 生成NanoId
    pub fn nanoid_len(&self, len: usize) -> String {
        nanoid::nanoid!(len)
    }

    /// Generate self-incrementing ID based on base62 code / 根据base62编码生成自增ID
    ///
    /// `BASE62` refers to Base64 encoding that does not contain `+`
    /// `-` .
    ///
    /// `BASE62` 指的是不包含 `+` `-` 的Base64编码.
    ///
    /// # Arguments
    ///
    /// * `str` - current string / 当前字符串
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use tardis::TardisFuns;
    /// assert_eq!(TardisFuns::field.incr_by_base62("abcd1").unwrap(), "abcd2");
    /// assert_eq!(TardisFuns::field.incr_by_base62("abcd12").unwrap(), "abcd13");
    /// assert_eq!(TardisFuns::field.incr_by_base62("abcd9").unwrap(), "abceA");
    /// assert_eq!(TardisFuns::field.incr_by_base62("azzz9").unwrap(), "azz0A");
    /// assert_eq!(TardisFuns::field.incr_by_base62("a9999").unwrap(), "bAAAA");
    /// assert!(TardisFuns::field.incr_by_base62("999").is_none());
    /// ```
    ///
    pub fn incr_by_base62(&self, str: &str) -> Option<String> {
        self.incr_by(str, BASE62)
    }

    /// Generate self-incrementing ID based on base36 code / 根据base36编码生成自增ID
    ///
    /// `BASE36` refers to Base64 encoding that does not contain `+` `-`
    /// `A-Z` .
    ///
    /// `BASE36` 指的是不包含 `+` `-` `A-Z` 的Base64编码.
    ///
    /// # Arguments
    ///
    /// * `str` - current string / 当前字符串
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use tardis::TardisFuns;
    /// assert_eq!(TardisFuns::field.incr_by_base36("abcd1").unwrap(), "abcd2");
    /// assert_eq!(TardisFuns::field.incr_by_base36("abcd12").unwrap(), "abcd13");
    /// assert_eq!(TardisFuns::field.incr_by_base36("abcd9").unwrap(), "abcea");
    /// assert_eq!(TardisFuns::field.incr_by_base36("azzz9").unwrap(), "azz0a");
    /// assert_eq!(TardisFuns::field.incr_by_base36("a9999").unwrap(), "baaaa");
    /// assert!(TardisFuns::field.incr_by_base36("999").is_none());
    /// ```
    ///
    pub fn incr_by_base36(&self, str: &str) -> Option<String> {
        self.incr_by(str, BASE36)
    }

    /// Using custom codes to generate self-incrementing ID / 使用自定义编码生成自增ID
    ///
    /// # Arguments
    ///
    /// * `str` - current string / 当前字符串
    /// * `chars` - custom encoded string / 自定义的编码字符串
    ///
    pub fn incr_by(&self, str: &str, chars: &str) -> Option<String> {
        let mut result = Vec::new();
        let mut up = true;
        for x in str.chars().rev() {
            if !up {
                result.push(x.to_string());
                continue;
            }
            let idx = chars.find(x).expect("[Tardis.Field] Invalid increment character");
            if idx == chars.len() - 1 {
                up = true;
                result.push(chars[..1].to_string());
            } else {
                up = false;
                result.push(chars[idx + 1..idx + 2].to_string());
            }
        }
        if !up {
            result.reverse();
            Some(result.join(""))
        } else {
            None
        }
    }
}

/// String types that support auto-trim / 支持自动trim的字符串类型
///
/// Valid by default when using [serde] serialization and deserialization.
///
/// 默认情况下，在使用 [serde] 序列化与反序列化时有效.
///
/// Valid when request body to Rust object when `web-server` feature is enabled.
///
/// 当启用 `web-server` feature时，在请求体转Rust对象时有效.
///
/// ```ignore
/// use serde::{Serialize,Deserialize};
/// use serde_json::Value::Object;
/// use tardis::basic::field::TrimString;
/// #[derive(Object, Serialize, Deserialize, Debug)]
/// struct TodoAddReq {
///     code: TrimString,
///     description: String,
///     done: bool,
/// }
/// ```
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct TrimString(pub String);

impl From<&str> for TrimString {
    fn from(str: &str) -> Self {
        TrimString(str.to_string())
    }
}

impl From<String> for TrimString {
    fn from(str: String) -> Self {
        TrimString(str)
    }
}

impl Clone for TrimString {
    fn clone(&self) -> Self {
        TrimString(self.0.clone())
    }
}

impl Display for TrimString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.trim(), f)
    }
}

impl Serialize for TrimString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for TrimString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(TrimString)
    }
}

impl AsRef<str> for TrimString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(feature = "web-server")]
impl crate::web::poem_openapi::types::Type for TrimString {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> std::borrow::Cow<'static, str> {
        "trim_string".into()
    }

    fn schema_ref() -> poem_openapi::registry::MetaSchemaRef {
        String::schema_ref()
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.as_raw_value().into_iter())
    }
}

#[cfg(feature = "web-server")]
impl crate::web::poem_openapi::types::ToJSON for TrimString {
    fn to_json(&self) -> Option<serde_json::Value> {
        self.0.to_json()
    }
}

#[cfg(feature = "web-server")]
impl crate::web::poem_openapi::types::ParseFromJSON for TrimString {
    fn parse_from_json(value: Option<serde_json::Value>) -> poem_openapi::types::ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let serde_json::Value::String(value) = value {
            Ok(TrimString(value))
        } else {
            Err(poem_openapi::types::ParseError::expected_type(value))
        }
    }
}
