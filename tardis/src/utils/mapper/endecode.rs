use crate::{basic::result::TardisResult, TardisFuns};

use super::Mapper;
pub struct Base64Encode;
pub struct Base64Decode;

impl Mapper<String> for Base64Encode {
    type Output = String;
    fn map(value: String) -> String {
        TardisFuns::crypto.base64.encode(value)
    }
}

impl<'a> Mapper<&'a str> for Base64Encode {
    type Output = String;
    fn map(value: &'a str) -> String {
        TardisFuns::crypto.base64.encode(value)
    }
}

impl Mapper<String> for Base64Decode {
    type Output = TardisResult<String>;
    fn map(value: String) -> TardisResult<String> {
        TardisFuns::crypto.base64.decode(value)
    }
}

impl<'a> Mapper<&'a str> for Base64Decode {
    type Output = TardisResult<String>;
    fn map(value: &'a str) -> TardisResult<String> {
        TardisFuns::crypto.base64.decode(value)
    }
}
