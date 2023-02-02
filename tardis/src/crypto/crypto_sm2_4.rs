use libsm::{
    sm2::ecc::Point,
    sm2::encrypt::{DecryptCtx, EncryptCtx},
    sm2::signature::{SigCtx, Signature},
    sm4::{Cipher, Mode},
};
use num_bigint::BigUint;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

pub struct TardisCryptoSm4;

pub struct TardisCryptoSm2;

pub struct TardisCryptoSm2PrivateKey {
    pri_key: BigUint,
}

pub struct TardisCryptoSm2PublicKey {
    pub_key: Point,
}

/// SM2 handle / SM2处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let private_key = TardisFuns::crypto.rsa.new_private_key().unwrap();
/// let public_key = TardisFuns::crypto.rsa.new_public_key(&private_key).unwrap();
///
/// let signed_data = private_key.sign("测试").unwrap();
/// public_key.verify("测试", &signed_data).unwrap();
///
/// let encrypted_data = public_key.encrypt("测试").unwrap();
/// private_key.decrypt(&encrypted_data).unwrap();
/// ```
#[cfg(feature = "crypto-with-sm")]
impl TardisCryptoSm2 {
    pub fn new_private_key(&self) -> TardisResult<TardisCryptoSm2PrivateKey> {
        TardisCryptoSm2PrivateKey::new()
    }

    pub fn new_private_key_from_str(&self, private_key: &str) -> TardisResult<TardisCryptoSm2PrivateKey> {
        TardisCryptoSm2PrivateKey::from(private_key)
    }

    pub fn new_public_key(&self, private_key: &TardisCryptoSm2PrivateKey) -> TardisResult<TardisCryptoSm2PublicKey> {
        TardisCryptoSm2PublicKey::from_private_key(private_key)
    }

    pub fn new_public_key_from_public_key(&self, public_key: &str) -> TardisResult<TardisCryptoSm2PublicKey> {
        TardisCryptoSm2PublicKey::from_public_key_str(public_key)
    }

    pub fn new_public_key_from_private_key(&self, private_key: &str) -> TardisResult<TardisCryptoSm2PublicKey> {
        TardisCryptoSm2PublicKey::from_private_key_str(private_key)
    }
}

#[cfg(feature = "crypto-with-sm")]
impl TardisCryptoSm2PrivateKey {
    pub fn new() -> TardisResult<Self> {
        let (_, sk) = SigCtx::new()
            .new_keypair()
            .map_err(|error| TardisError::internal_error(&format!("[Tardis.Crypto] SM2 new keypair error:{error}"), "500-tardis-crypto-sm2-keypair-error"))?;
        Ok(TardisCryptoSm2PrivateKey { pri_key: sk })
    }

    pub fn from(private_key: &str) -> TardisResult<Self> {
        let sk = SigCtx::new()
            .load_seckey(&hex::decode(private_key)?)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 load sk error:{error}"), "406-tardis-crypto-sm2-sk-error"))?;
        Ok(TardisCryptoSm2PrivateKey { pri_key: sk })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        let sk = SigCtx::new()
            .serialize_seckey(&self.pri_key)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 serialize sk error:{error}"), "406-tardis-crypto-sm2-sk-error"))?;
        Ok(hex::encode(sk))
    }

    pub fn decrypt(&self, encrypted_data: &str) -> TardisResult<String> {
        let encrypted_data = hex::decode(encrypted_data)?;
        // https://github.com/citahub/libsm/issues/46
        let data = DecryptCtx::new(encrypted_data.len() - 97, self.pri_key.clone())
            .decrypt(&encrypted_data)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 decrypt error:{error}"), "406-tardis-crypto-sm2-decrypt-error"))?;
        Ok(String::from_utf8(data)?)
    }

    pub fn sign(&self, data: &str) -> TardisResult<String> {
        let ctx = SigCtx::new();
        let pk =
            ctx.pk_from_sk(&self.pri_key).map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 get pk error:{error}"), "406-tardis-crypto-sm2-pk-error"))?;
        let signature = ctx
            .sign(data.as_bytes(), &self.pri_key, &pk)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 sign error:{error}"), "406-tardis-crypto-sm2-sign-error"))?;
        Ok(hex::encode(signature.der_encode()))
    }
}

#[cfg(feature = "crypto-with-sm")]
impl TardisCryptoSm2PublicKey {
    pub fn from_private_key(private_key: &TardisCryptoSm2PrivateKey) -> TardisResult<Self> {
        let pk = SigCtx::new()
            .pk_from_sk(&private_key.pri_key)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 get pk error:{error}"), "406-tardis-crypto-sm2-pk-error"))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn from_private_key_str(private_key: &str) -> TardisResult<Self> {
        let ctx = SigCtx::new();
        let sk = ctx
            .load_seckey(&hex::decode(private_key)?)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 load sk error:{error}"), "406-tardis-crypto-sm2-sk-error"))?;
        let pk = ctx.pk_from_sk(&sk).map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 get pk error:{error}"), "406-tardis-crypto-sm2-pk-error"))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn from_public_key_str(public_key: &str) -> TardisResult<Self> {
        let pk = SigCtx::new()
            .load_pubkey(&hex::decode(public_key)?)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 load pk error:{error}"), "406-tardis-crypto-sm2-pk-error"))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        let pk = SigCtx::new()
            .serialize_pubkey(&self.pub_key, true)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 serialize pk error:{error}"), "406-tardis-crypto-sm2-pk-error"))?;
        Ok(hex::encode(pk))
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let encrypted_data = EncryptCtx::new(data.len(), self.pub_key)
            .encrypt(data.as_bytes())
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 encrypt error:{error}"), "406-tardis-crypto-sm2-encrypt-error"))?;
        Ok(hex::encode(encrypted_data))
    }

    pub fn verify(&self, data: &str, signed_data: &str) -> TardisResult<bool> {
        let signed_data = hex::decode(signed_data)?;
        let signature = Signature::der_decode(&signed_data)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 decode signature error:{error}"), "406-tardis-crypto-sm2-decode-sign-error"))?;
        let result = SigCtx::new()
            .verify(data.as_bytes(), &self.pub_key, &signature)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM2 verify error:{error}"), "406-tardis-crypto-sm2-verify-sign-error"))?;
        Ok(result)
    }
}

/// SM4 handle / SM4处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let key = TardisFuns::crypto.key.rand_16_hex().unwrap();
/// let iv = TardisFuns::crypto.key.rand_16_hex().unwrap();
/// let text = "为什么选择 Rust?";
/// let encrypted_data = TardisFuns::crypto.sm4.encrypt_cbc(text, &key, &iv).unwrap();
/// let data = TardisFuns::crypto.sm4.decrypt_cbc(&encrypted_data, &key, &iv).unwrap();
/// ```
#[cfg(feature = "crypto-with-sm")]
impl TardisCryptoSm4 {
    pub fn encrypt_cbc(&self, data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let cipher = Cipher::new(hex_key.as_bytes(), Mode::Cbc)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM4 new cipher error:{error}"), "406-tardis-crypto-sm4-cipher-error"))?;
        let encrypted_data = cipher
            .encrypt(data.as_bytes(), hex_iv.as_bytes())
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM4 encrypt error:{error}"), "406-tardis-crypto-sm4-encrypt-error"))?;
        Ok(hex::encode(encrypted_data))
    }

    pub fn decrypt_cbc(&self, encrypted_data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let cipher = Cipher::new(hex_key.as_bytes(), Mode::Cbc)
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM4 new cipher error:{error}"), "406-tardis-crypto-sm4-cipher-error"))?;
        let encrypted_data = hex::decode(encrypted_data)?;
        let data = cipher
            .decrypt(encrypted_data.as_slice(), hex_iv.as_bytes())
            .map_err(|error| TardisError::format_error(&format!("[Tardis.Crypto] SM4 decrypt error:{error}"), "406-tardis-crypto-sm4-decrypt-error"))?;
        Ok(String::from_utf8(data)?)
    }
}
