use std::fmt::{Display, Formatter};

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

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

pub struct TardisField;

impl TardisField {
    pub fn is_phone(&self, phone: &str) -> bool {
        R_PHONE.is_match(phone)
    }

    pub fn is_mail(&self, mail: &str) -> bool {
        R_MAIL.is_match(mail)
    }

    pub fn is_code_cs(&self, str: &str) -> bool {
        R_CODE_CS.is_match(str)
    }

    pub fn is_code_ncs(&self, str: &str) -> bool {
        R_CODE_NCS.is_match(str)
    }

    pub fn uuid(&self) -> Uuid {
        uuid::Uuid::new_v4()
    }

    pub fn uuid_str(&self) -> String {
        uuid::Uuid::new_v4().to_simple().to_string()
    }

    pub fn incr_by_base62(&self, str: &str) -> Option<String> {
        self.incr_by(str, BASE62)
    }

    pub fn incr_by_base36(&self, str: &str) -> Option<String> {
        self.incr_by(str, BASE36)
    }

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

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct TrimString(String);

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
