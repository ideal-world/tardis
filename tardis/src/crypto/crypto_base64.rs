use base64::engine::general_purpose;
use base64::Engine;

use crate::basic::dto::TardisContext;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::utils::mapper::{Base64Decode, Base64Encode, Mapper};
use crate::TardisFuns;
pub struct TardisCryptoBase64;

impl TardisCryptoBase64 {
    pub fn decode(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        match general_purpose::STANDARD.decode(data) {
            Ok(result) => Ok(String::from_utf8(result)?),
            Err(error) => Err(TardisError::format_error(
                &format!("[Tardis.Crypto] Base64 decode error:{error}"),
                "406-tardis-crypto-base64-decode-error",
            )),
        }
    }

    pub fn encode(&self, data: impl AsRef<[u8]>) -> String {
        general_purpose::STANDARD.encode(data)
    }

    pub fn encode_raw<T: AsRef<[u8]>>(&self, data: T) -> String {
        general_purpose::STANDARD.encode(data)
    }
}

impl TardisContext {
    pub fn to_base64(&self) -> TardisResult<String> {
        let ctx = TardisContext::default();
        let ctx = TardisFuns::json.obj_to_string(&ctx)?;
        Ok(TardisFuns::crypto.base64.encode(ctx))
    }
}

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
