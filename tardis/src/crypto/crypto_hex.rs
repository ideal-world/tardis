use crate::basic::{error::TardisError, result::TardisResult};

/// # TardisCryptoHex
/// Encode and decode with hex.
pub struct TardisCryptoHex;

impl TardisCryptoHex {
    /// decode from hex to raw binary data
    pub fn decode<T: AsRef<[u8]>>(&self, data: T) -> TardisResult<Vec<u8>> {
        match hex::decode(data) {
            Ok(result) => Ok(result),
            Err(error) => Err(TardisError::format_error(
                &format!("[Tardis.Crypto] Hex decode error:{error}"),
                "406-tardis-crypto-hex-decode-error",
            )),
        }
    }

    /// encode to hex
    pub fn encode<T: AsRef<[u8]>>(&self, data: T) -> String {
        hex::encode(data)
    }
}
