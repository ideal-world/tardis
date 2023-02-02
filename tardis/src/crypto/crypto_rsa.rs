use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::PublicKey;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;
pub struct TardisCryptoRsa;

pub struct TardisCryptoRsaPrivateKey {
    pri_key: rsa::RsaPrivateKey,
}

pub struct TardisCryptoRsaPublicKey {
    pub_key: rsa::RsaPublicKey,
}

/// RSA handle / RSA处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let private_key = TardisFuns::crypto.rsa.new_private_key(2048).unwrap();
/// let public_key = TardisFuns::crypto.rsa.new_public_key(&private_key).unwrap();
///
/// let signed_data = private_key.sign("测试").unwrap();
/// public_key.verify("测试", &signed_data).unwrap();
///
/// let encrypted_data = public_key.encrypt("测试").unwrap();
/// private_key.decrypt(&encrypted_data).unwrap();
/// ```
impl TardisCryptoRsa {
    pub fn new_private_key(&self, bits: usize) -> TardisResult<TardisCryptoRsaPrivateKey> {
        TardisCryptoRsaPrivateKey::new(bits)
    }

    pub fn new_private_key_from_str(&self, private_key_pem: &str) -> TardisResult<TardisCryptoRsaPrivateKey> {
        TardisCryptoRsaPrivateKey::from(private_key_pem)
    }

    pub fn new_public_key(&self, private_key: &TardisCryptoRsaPrivateKey) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_private_key(private_key)
    }

    pub fn new_public_key_from_public_key(&self, public_key_pem: &str) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_public_key_str(public_key_pem)
    }

    pub fn new_public_key_from_private_key(&self, private_key_pem: &str) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_private_key_str(private_key_pem)
    }
}

impl TardisCryptoRsaPrivateKey {
    pub fn new(bits: usize) -> TardisResult<Self> {
        let mut rand = rand::rngs::OsRng;
        Ok(TardisCryptoRsaPrivateKey {
            pri_key: rsa::RsaPrivateKey::new(&mut rand, bits)?,
        })
    }

    pub fn from(private_key_pem: &str) -> TardisResult<Self> {
        Ok(TardisCryptoRsaPrivateKey {
            pri_key: rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)
                .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto sk load error, {error}"), "406-tardis-crypto-rsa-sk-error"))?,
        })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        Ok(self
            .pri_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto sk serialize error, {error}"), "406-tardis-crypto-rsa-sk-error"))?
            .to_string())
    }

    pub fn decrypt(&self, encrypted_data: &str) -> TardisResult<String> {
        let encrypted_data = hex::decode(encrypted_data)?;
        let data = self.pri_key.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, encrypted_data.as_slice())?;
        Ok(String::from_utf8(data)?)
    }

    pub fn sign(&self, data: &str) -> TardisResult<String> {
        let signed_data = self.pri_key.sign(
            rsa::PaddingScheme::PKCS1v15Sign {
                hash_len: None,
                prefix: Box::new([]),
            },
            TardisFuns::crypto.digest.sha256(data)?.as_bytes(),
        )?;
        Ok(hex::encode(signed_data))
    }
}

impl TardisCryptoRsaPublicKey {
    pub fn from_private_key(private_key: &TardisCryptoRsaPrivateKey) -> TardisResult<Self> {
        let public_key = rsa::RsaPublicKey::from(&private_key.pri_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_private_key_str(private_key_pem: &str) -> TardisResult<Self> {
        let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto sk load error, {error}"), "406-tardis-crypto-rsa-sk-error"))?;
        let public_key = rsa::RsaPublicKey::from(private_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_public_key_str(public_key_pem: &str) -> TardisResult<Self> {
        Ok(TardisCryptoRsaPublicKey {
            pub_key: rsa::RsaPublicKey::from_public_key_pem(public_key_pem)
                .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto pk load error, {error}"), "406-tardis-crypto-rsa-pk-error"))?,
        })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        self.pub_key
            .to_public_key_pem(LineEnding::LF)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto pk serialize error, {error}"), "406-tardis-crypto-rsa-pk-error"))
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let mut rand = rand::rngs::OsRng;
        let encrypted_data = self.pub_key.encrypt(&mut rand, rsa::PaddingScheme::PKCS1v15Encrypt, data.as_bytes())?;
        Ok(hex::encode(encrypted_data))
    }

    pub fn verify(&self, data: &str, signed_data: &str) -> TardisResult<bool> {
        let signed_data = hex::decode(signed_data)?;
        let result = self.pub_key.verify(
            rsa::PaddingScheme::PKCS1v15Sign {
                hash_len: None,
                prefix: Box::new([]),
            },
            TardisFuns::crypto.digest.sha256(data)?.as_bytes(),
            signed_data.as_slice(),
        );
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl From<rsa::errors::Error> for TardisError {
    fn from(error: rsa::errors::Error) -> Self {
        TardisError::format_error(&format!("[Tardis.Crypto] RSA crypto error, {error:?}"), "406-tardis-crypto-rsa-error")
    }
}
