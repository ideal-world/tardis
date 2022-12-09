use super::{
    crypto_aes::TardisCryptoAes, crypto_base64::TardisCryptoBase64, crypto_digest::TardisCryptoDigest, crypto_hex::TardisCryptoHex, crypto_key::TardisCryptoKey,
    crypto_rsa::TardisCryptoRsa,
};

/// Crypto handle / 加解密处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let b64_str = TardisFuns::crypto.base64.encode("测试");
/// let str = TardisFuns::crypto.base64.decode(&b64_str).unwrap();
/// ```
pub struct TardisCrypto {
    pub key: TardisCryptoKey,
    pub hex: TardisCryptoHex,
    pub base64: TardisCryptoBase64,
    pub aes: TardisCryptoAes,
    pub rsa: TardisCryptoRsa,
    pub digest: TardisCryptoDigest,
    #[cfg(feature = "crypto-with-sm")]
    pub sm4: super::crypto_sm2_4::TardisCryptoSm4,
    #[cfg(feature = "crypto-with-sm")]
    pub sm2: super::crypto_sm2_4::TardisCryptoSm2,
}
