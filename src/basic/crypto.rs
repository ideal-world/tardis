use crypto::{
    buffer::{ReadBuffer, WriteBuffer},
    mac::Mac,
};
#[cfg(feature = "crypto_with_sm")]
use libsm::{
    sm2::ecc::Point,
    sm2::encrypt::{DecryptCtx, EncryptCtx},
    sm2::signature::{SigCtx, Signature},
    sm3::hash::Sm3Hash,
    sm4::{Cipher, Mode},
};
#[cfg(feature = "crypto_with_sm")]
use num_bigint::BigUint;
use rand_core::RngCore;
use rsa::pkcs8::{FromPrivateKey, FromPublicKey, ToPrivateKey, ToPublicKey};
use rsa::{pkcs8, PublicKey};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;

/// Base64 handle / Base64处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let b64_str = TardisFuns::crypto.base64.encode("测试");
/// let str = TardisFuns::crypto.base64.decode(&b64_str).unwrap();
/// ```
pub struct TardisCrypto {
    pub key: TardisCryptoKey,
    pub base64: TardisCryptoBase64,
    pub aes: TardisCryptoAes,
    pub rsa: TardisCryptoRsa,
    pub digest: TardisCryptoDigest,
    #[cfg(feature = "crypto_with_sm")]
    pub sm4: TardisCryptoSm4,
    #[cfg(feature = "crypto_with_sm")]
    pub sm2: TardisCryptoSm2,
}
pub struct TardisCryptoBase64;
pub struct TardisCryptoAes;
pub struct TardisCryptoRsa;
pub struct TardisCryptoRsaPrivateKey {
    pri_key: rsa::RsaPrivateKey,
}
pub struct TardisCryptoRsaPublicKey {
    pub_key: rsa::RsaPublicKey,
}
#[cfg(feature = "crypto_with_sm")]
pub struct TardisCryptoSm4;
#[cfg(feature = "crypto_with_sm")]
pub struct TardisCryptoSm2;
#[cfg(feature = "crypto_with_sm")]
pub struct TardisCryptoSm2PrivateKey {
    pri_key: BigUint,
}
#[cfg(feature = "crypto_with_sm")]
pub struct TardisCryptoSm2PublicKey {
    pub_key: Point,
}
pub struct TardisCryptoDigest;
pub struct TardisCryptoKey;

impl TardisCryptoBase64 {
    pub fn decode(&self, data: &str) -> TardisResult<String> {
        match base64::decode(data) {
            Ok(result) => Ok(String::from_utf8(result)?),
            Err(e) => Err(TardisError::FormatError(format!("[Tardis.Crypto] Base64 decode error:{}", e))),
        }
    }

    pub fn encode(&self, data: &str) -> String {
        base64::encode(data)
    }
}

/// AES handle / AES处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// let key = TardisFuns::crypto.key.rand_16_hex().unwrap();
/// let iv = TardisFuns::crypto.key.rand_16_hex().unwrap();
/// let text = "为什么选择 Rust?";
/// let encrypted_data = TardisFuns::crypto.aes.encrypt_cbc(text, &key, &iv).unwrap();
/// let data = TardisFuns::crypto.aes.decrypt_cbc(&encrypted_data, &key, &iv).unwrap();
/// ```
impl TardisCryptoAes {
    pub fn encrypt_cbc(&self, data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let key_size = match hex_key.len() {
            32 => crypto::aes::KeySize::KeySize128,
            64 => crypto::aes::KeySize::KeySize256,
            _ => return Err(TardisError::BadRequest("[Tardis.Crypto] AES error, invalid key size".to_string())),
        };

        let mut encryptor = crypto::aes::cbc_encryptor(key_size, hex::decode(hex_key)?.as_slice(), hex::decode(hex_iv)?.as_slice(), crypto::blockmodes::PkcsPadding);

        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = crypto::buffer::RefReadBuffer::new(data.as_bytes());
        let mut buffer = [0; 4096];
        let mut write_buffer = crypto::buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true).unwrap();
            final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());
            match result {
                crypto::buffer::BufferResult::BufferUnderflow => break,
                crypto::buffer::BufferResult::BufferOverflow => {}
            }
        }
        Ok(base64::encode(&final_result))
    }

    pub fn decrypt_cbc(&self, encrypted_data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let key_size = match hex_key.len() {
            32 => crypto::aes::KeySize::KeySize128,
            64 => crypto::aes::KeySize::KeySize256,
            _ => return Err(TardisError::BadRequest("[Tardis.Crypto] AES error, invalid key size".to_string())),
        };

        let encrypted_data = base64::decode(encrypted_data)?;

        let mut decryptor = crypto::aes::cbc_decryptor(key_size, hex::decode(hex_key)?.as_slice(), hex::decode(hex_iv)?.as_slice(), crypto::blockmodes::PkcsPadding);

        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = crypto::buffer::RefReadBuffer::new(encrypted_data.as_slice());
        let mut buffer = [0; 4096];
        let mut write_buffer = crypto::buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
            final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());
            match result {
                crypto::buffer::BufferResult::BufferUnderflow => break,
                crypto::buffer::BufferResult::BufferOverflow => {}
            }
        }

        Ok(String::from_utf8(final_result)?)
    }
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
            pri_key: rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?,
        })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        Ok(self.pri_key.to_pkcs8_pem()?.to_string())
    }

    pub fn decrypt(&self, encrypted_data: &str) -> TardisResult<String> {
        let encrypted_data = base64::decode(encrypted_data)?;
        let data = self.pri_key.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, encrypted_data.as_slice())?;
        Ok(String::from_utf8(data)?)
    }

    pub fn sign(&self, data: &str) -> TardisResult<String> {
        let signed_data = self.pri_key.sign(rsa::PaddingScheme::PKCS1v15Sign { hash: None }, TardisFuns::crypto.digest.sha256(data)?.as_bytes())?;
        Ok(base64::encode(&signed_data))
    }
}

impl TardisCryptoRsaPublicKey {
    pub fn from_private_key(private_key: &TardisCryptoRsaPrivateKey) -> TardisResult<Self> {
        let public_key = rsa::RsaPublicKey::from(&private_key.pri_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_private_key_str(private_key_pem: &str) -> TardisResult<Self> {
        let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
        let public_key = rsa::RsaPublicKey::from(private_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_public_key_str(public_key_pem: &str) -> TardisResult<Self> {
        Ok(TardisCryptoRsaPublicKey {
            pub_key: rsa::RsaPublicKey::from_public_key_pem(public_key_pem)?,
        })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        Ok(self.pub_key.to_public_key_pem()?)
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let mut rand = rand::rngs::OsRng;
        let encrypted_data = self.pub_key.encrypt(&mut rand, rsa::PaddingScheme::PKCS1v15Encrypt, data.as_bytes())?;
        Ok(base64::encode(&encrypted_data))
    }

    pub fn verify(&self, data: &str, signed_data: &str) -> TardisResult<bool> {
        let signed_data = base64::decode(signed_data)?;
        let result = self.pub_key.verify(
            rsa::PaddingScheme::PKCS1v15Sign { hash: None },
            TardisFuns::crypto.digest.sha256(data)?.as_bytes(),
            signed_data.as_slice(),
        );
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
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
#[cfg(feature = "crypto_with_sm")]
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

#[cfg(feature = "crypto_with_sm")]
impl TardisCryptoSm2PrivateKey {
    pub fn new() -> TardisResult<Self> {
        let (_, sk) = SigCtx::new().new_keypair().map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 new keypair error:{}", e)))?;
        Ok(TardisCryptoSm2PrivateKey { pri_key: sk })
    }

    pub fn from(private_key: &str) -> TardisResult<Self> {
        let sk = SigCtx::new().load_seckey(&base64::decode(private_key)?).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 load sk error:{}", e)))?;
        Ok(TardisCryptoSm2PrivateKey { pri_key: sk })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        let sk = SigCtx::new().serialize_seckey(&self.pri_key).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 serialize sk error:{}", e)))?;
        Ok(base64::encode(&sk))
    }

    pub fn decrypt(&self, encrypted_data: &str) -> TardisResult<String> {
        let encrypted_data = base64::decode(encrypted_data)?;
        // https://github.com/citahub/libsm/issues/46
        let data = DecryptCtx::new(encrypted_data.len() - 97, self.pri_key.clone())
            .decrypt(&encrypted_data)
            .map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 decrypt error:{}", e)))?;
        Ok(String::from_utf8(data)?)
    }

    pub fn sign(&self, data: &str) -> TardisResult<String> {
        let ctx = SigCtx::new();
        let pk = ctx.pk_from_sk(&self.pri_key).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 get pk error:{}", e)))?;
        let signature = ctx.sign(data.as_bytes(), &self.pri_key, &pk).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 sign error:{}", e)))?;
        Ok(base64::encode(&signature.der_encode()))
    }
}

#[cfg(feature = "crypto_with_sm")]
impl TardisCryptoSm2PublicKey {
    pub fn from_private_key(private_key: &TardisCryptoSm2PrivateKey) -> TardisResult<Self> {
        let pk = SigCtx::new().pk_from_sk(&private_key.pri_key).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 get pk error:{}", e)))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn from_private_key_str(private_key: &str) -> TardisResult<Self> {
        let ctx = SigCtx::new();
        let sk = ctx.load_seckey(&base64::decode(private_key)?).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 load sk error:{}", e)))?;
        let pk = ctx.pk_from_sk(&sk).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 get pk error:{}", e)))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn from_public_key_str(public_key: &str) -> TardisResult<Self> {
        let pk = SigCtx::new().load_pubkey(&base64::decode(public_key)?).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 load pk error:{}", e)))?;
        Ok(TardisCryptoSm2PublicKey { pub_key: pk })
    }

    pub fn serialize(&self) -> TardisResult<String> {
        let pk = SigCtx::new().serialize_pubkey(&self.pub_key, true).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 serialize pk error:{}", e)))?;
        Ok(base64::encode(&pk))
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let encrypted_data =
            EncryptCtx::new(data.len(), self.pub_key).encrypt(data.as_bytes()).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 encrypt error:{}", e)))?;
        Ok(base64::encode(&encrypted_data))
    }

    pub fn verify(&self, data: &str, signed_data: &str) -> TardisResult<bool> {
        let signed_data = base64::decode(signed_data)?;
        let signature = Signature::der_decode(&signed_data).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 decode signature error:{}", e)))?;
        let result = SigCtx::new().verify(data.as_bytes(), &self.pub_key, &signature).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM2 verify error:{}", e)))?;
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
#[cfg(feature = "crypto_with_sm")]
impl TardisCryptoSm4 {
    pub fn encrypt_cbc(&self, data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let cipher = Cipher::new(hex::decode(hex_key)?.as_slice(), Mode::Cbc).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM4 new cipher error:{}", e)))?;
        let encrypted_data =
            cipher.encrypt(data.as_bytes(), hex::decode(hex_iv)?.as_slice()).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM4 encrypt error:{}", e)))?;
        Ok(base64::encode(&encrypted_data))
    }

    pub fn decrypt_cbc(&self, encrypted_data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        let cipher = Cipher::new(hex::decode(hex_key)?.as_slice(), Mode::Cbc).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM4 new cipher error:{}", e)))?;
        let encrypted_data = base64::decode(encrypted_data)?;
        let data =
            cipher.decrypt(encrypted_data.as_slice(), hex::decode(hex_iv)?.as_slice()).map_err(|e| TardisError::FormatError(format!("[Tardis.Crypto] SM4 decrypt error:{}", e)))?;
        Ok(String::from_utf8(data)?)
    }
}

/// Digest handle / 摘要处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// TardisFuns::crypto.digest.md5("测试").unwrap();
/// TardisFuns::crypto.digest.sha1("测试").unwrap();
/// TardisFuns::crypto.digest.sha256("测试").unwrap();
/// TardisFuns::crypto.digest.sha512("测试").unwrap();
///
/// TardisFuns::crypto.digest.hmac_sha1("测试", "pwd").unwrap();
/// TardisFuns::crypto.digest.hmac_sha256("测试", "pwd").unwrap();
/// TardisFuns::crypto.digest.hmac_sha512("测试", "pwd").unwrap();
///
/// TardisFuns::crypto.digest.sm3("测试").unwrap();
/// ```
impl TardisCryptoDigest {
    pub fn sha1(&self, data: &str) -> TardisResult<String> {
        self.digest(data, crypto::sha1::Sha1::new())
    }

    pub fn sha256(&self, data: &str) -> TardisResult<String> {
        self.digest(data, crypto::sha2::Sha256::new())
    }

    pub fn sha512(&self, data: &str) -> TardisResult<String> {
        self.digest(data, crypto::sha2::Sha512::new())
    }

    pub fn md5(&self, data: &str) -> TardisResult<String> {
        self.digest(data, crypto::md5::Md5::new())
    }

    pub fn hmac_sha1(&self, data: &str, key: &str) -> TardisResult<String> {
        self.digest_hmac(data, key, crypto::sha1::Sha1::new())
    }

    pub fn hmac_sha256(&self, data: &str, key: &str) -> TardisResult<String> {
        self.digest_hmac(data, key, crypto::sha2::Sha256::new())
    }

    pub fn hmac_sha512(&self, data: &str, key: &str) -> TardisResult<String> {
        self.digest_hmac(data, key, crypto::sha2::Sha512::new())
    }

    #[cfg(feature = "crypto_with_sm")]
    pub fn sm3(&self, data: &str) -> TardisResult<String> {
        Ok(hex::encode(Sm3Hash::new(data.as_bytes()).get_hash()))
    }

    fn digest<A: crypto::digest::Digest>(&self, data: &str, mut algorithm: A) -> TardisResult<String> {
        algorithm.input_str(data);
        Ok(algorithm.result_str())
    }

    fn digest_hmac<A: crypto::digest::Digest>(&self, data: &str, key: &str, algorithm: A) -> TardisResult<String> {
        let mut hmac = crypto::hmac::Hmac::new(algorithm, key.as_bytes());
        hmac.input(data.as_bytes());
        Ok(base64::encode(hmac.result().code()))
    }
}

impl TardisCryptoKey {
    pub fn rand_8_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 8] = [0; 8];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_16_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 16] = [0; 16];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_32_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 32] = [0; 32];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_64_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 64] = [0; 64];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_128_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 128] = [0; 128];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn generate_token(&self) -> TardisResult<String> {
        Ok(format!("tk{}", TardisFuns::field.nanoid()))
    }

    pub fn generate_ak(&self) -> TardisResult<String> {
        Ok(format!("ak{}", TardisFuns::field.nanoid()))
    }

    pub fn generate_sk(&self, ak: &str) -> TardisResult<String> {
        let sk = TardisFuns::crypto.digest.sha1(format!("{}{}", ak, TardisFuns::field.nanoid()).as_str());
        match sk {
            Ok(sk) => Ok(sk),
            Err(e) => Err(e),
        }
    }
}

impl From<crypto::symmetriccipher::SymmetricCipherError> for TardisError {
    fn from(error: crypto::symmetriccipher::SymmetricCipherError) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] AES crypto error, {:?}", error))
    }
}

impl From<rsa::errors::Error> for TardisError {
    fn from(error: rsa::errors::Error) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] RSA crypto error, {}", error))
    }
}

impl From<pkcs8::Error> for TardisError {
    fn from(error: pkcs8::Error) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] RSA crypto error, {}", error))
    }
}
