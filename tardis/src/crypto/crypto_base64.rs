use base64::engine::general_purpose;
use base64::Engine;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
pub struct TardisCryptoBase64;

impl TardisCryptoBase64 {
    pub fn decode(&self, data: &str) -> TardisResult<String> {
        match general_purpose::STANDARD.decode(data) {
            Ok(result) => Ok(String::from_utf8(result)?),
            Err(error) => Err(TardisError::format_error(
                &format!("[Tardis.Crypto] Base64 decode error:{error}"),
                "406-tardis-crypto-base64-decode-error",
            )),
        }
    }

    pub fn encode(&self, data: &str) -> String {
        general_purpose::STANDARD.encode(data)
    }
}
