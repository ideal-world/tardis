use regex::Regex;

use crate::{
    tardis_static,
    utils::mapper::{Base64Decode, Base64Encode, Mapped, Trim},
};

tardis_static! {
    pub r_phone: Regex = Regex::new(r"^1(3\d|4[5-9]|5[0-35-9]|6[2567]|7[0-8]|8\d|9[0-35-9])\d{8}$").expect("Regular parsing error");
    pub r_mail: Regex = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$")
        .expect("Regular parsing error");
    pub r_code_ncs: Regex = Regex::new(r"^[a-z0-9_]+$").expect("Regular parsing error");
    pub r_code_cs: Regex = Regex::new(r"^[A-Za-z0-9_]+$").expect("Regular parsing error");
}

static BASE62: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
static BASE36: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

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
/// assert!(TardisFuns::field.incr_by_base62("zzz").is_none());
/// assert_eq!(TardisFuns::field.incr_by_base36("abcd1").unwrap(), "abcd2");
/// assert!(TardisFuns::field.incr_by_base36("zzz").is_none());
/// assert!(TardisFuns::field.is_code_cs("Adw834_dfds"));
/// assert!(!TardisFuns::field.is_code_cs(" Adw834_dfds"));
/// assert!(!TardisFuns::field.is_code_cs("Adw834_d-fds"));
/// assert!(TardisFuns::field.is_code_ncs("adon2_43323tr"));
/// assert!(!TardisFuns::field.is_code_ncs("adon2_43323tr "));
/// assert!(!TardisFuns::field.is_code_ncs("Adw834_dfds"));
/// assert_eq!(TardisFuns::field.nanoid().len(), 21);
/// assert_eq!(TardisFuns::field.nanoid_len(4).len(), 4);
/// ```
#[allow(clippy::module_name_repetitions)]
pub struct TardisField;

impl TardisField {
    /// Determine if it is a cell phone number (only supports mainland China) / 判断是否是手机号（仅支持中国大陆）
    pub fn is_phone(&self, phone: &str) -> bool {
        r_phone().is_match(phone)
    }
    /// Determine if it is a email / 判断是否是邮箱
    pub fn is_mail(&self, mail: &str) -> bool {
        r_mail().is_match(mail)
    }

    /// Determine if it contains only numbers, lowercase letters and underscores /
    /// 判断是否只包含数字、小写字母及下划线
    pub fn is_code_cs(&self, str: &str) -> bool {
        r_code_cs().is_match(str)
    }

    /// Determine if only numbers, upper and lower case letters and underscores are included /
    /// 判断是否只包含数字、大小写字母及下划线
    pub fn is_code_ncs(&self, str: &str) -> bool {
        r_code_ncs().is_match(str)
    }

    /// Generate NanoId / 生成NanoId
    pub fn nanoid(&self) -> String {
        nanoid::nanoid!()
    }

    /// Generate NanoId / 生成NanoId
    pub fn nanoid_len(&self, len: usize) -> String {
        nanoid::nanoid!(len)
    }

    /// Generate NanoId / 生成NanoId
    pub fn nanoid_custom(&self, len: usize, alphabet: &[char]) -> String {
        nanoid::nanoid!(len, alphabet)
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
    /// assert_eq!(TardisFuns::field.incr_by_base62("abcd9").unwrap(), "abcdA");
    /// assert_eq!(TardisFuns::field.incr_by_base62("abcdz").unwrap(), "abce0");
    /// assert_eq!(TardisFuns::field.incr_by_base62("azZzz").unwrap(), "aza00");
    /// assert_eq!(TardisFuns::field.incr_by_base62("azzzz").unwrap(), "b0000");
    /// assert!(TardisFuns::field.incr_by_base62("zzz").is_none());
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
    /// assert_eq!(TardisFuns::field.incr_by_base36("abcd9").unwrap(), "abcda");
    /// assert_eq!(TardisFuns::field.incr_by_base36("0000").unwrap(), "0001");
    /// assert_eq!(TardisFuns::field.incr_by_base36("000z").unwrap(), "0010");
    /// assert_eq!(TardisFuns::field.incr_by_base36("azzzy").unwrap(), "azzzz");
    /// assert_eq!(TardisFuns::field.incr_by_base36("azzzz").unwrap(), "b0000");
    /// assert!(TardisFuns::field.incr_by_base36("zzz").is_none());
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
        if chars.len() <= 1 {
            return None;
        }
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
        if up {
            None
        } else {
            result.reverse();
            Some(result.join(""))
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
/// # Deref
///
/// `TrimString` implements <code>[Deref]<Target = [str]></code>
/// In addition, this means that you can pass a `TrimString` to a
/// function which takes a [`&str`] by using an ampersand (`&`):
///
/// TrimString类似String实现了<code>[Deref]<Target = [str]></code>，
/// 从而可以使用把`&`加在`TrimString`前面传入接受[`&str`]的函数：
/// ```
/// use tardis::basic::field::TrimString;
/// fn takes_str(s: &str) { }
///
/// let s = TrimString::from("Hello");
///
/// takes_str(&s);
/// ```
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
pub type TrimString = Mapped<String, Trim>;

/// This function is `non_snake_case` for being compatible with the old version
/// # Deprecated
/// Please use `TrimString::new` instead
#[allow(non_snake_case)]
// #[deprecated(since = "1.0.0", note = "Please use `TrimString::new` instead")]
pub fn TrimString(string: impl Into<String>) -> TrimString {
    TrimString::new(string.into())
}

impl From<&str> for TrimString {
    fn from(str: &str) -> Self {
        TrimString::new(str.to_string())
    }
}

impl AsRef<str> for TrimString {
    fn as_ref(&self) -> &str {
        self
    }
}

pub type TrimStr<'a> = Mapped<&'a str, Trim>;
pub type Base64EncodedString = Mapped<String, Base64Encode>;
pub type Base64DecodedString = Mapped<String, Base64Decode>;
