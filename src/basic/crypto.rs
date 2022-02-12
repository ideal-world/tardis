use crypto::{
    buffer::{ReadBuffer, WriteBuffer},
    mac::Mac,
};
use pkcs8::{FromPrivateKey, FromPublicKey, ToPrivateKey, ToPublicKey};
use rand_core::RngCore;
use rsa::PublicKey;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;

pub struct TardisCrypto {
    pub base64: TardisCryptoBase64,
    pub aes: TardisCryptoAes,
    pub rsa: TardisCryptoRsa,
    pub digest: TardisCryptoDigest,
    pub key: TardisCryptoKey,
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
pub struct TardisCryptoDigest;
pub struct TardisCryptoKey;

impl TardisCryptoBase64 {
    pub fn decode(&self, data: &str) -> TardisResult<String> {
        match base64::decode(data) {
            Ok(result) => Ok(String::from_utf8(result)?),
            Err(e) => Err(TardisError::FormatError(e.to_string())),
        }
    }

    pub fn encode(&self, data: &str) -> String {
        base64::encode(data)
    }
}

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
        Ok(base64::encode(final_result))
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

impl TardisCryptoRsa {
    pub fn new_private_key(&self, bits: usize) -> TardisResult<TardisCryptoRsaPrivateKey> {
        TardisCryptoRsaPrivateKey::new(bits)
    }

    pub fn new_private_key_from_pem(&self, private_key_pem: &str) -> TardisResult<TardisCryptoRsaPrivateKey> {
        TardisCryptoRsaPrivateKey::from_private_key_pem(private_key_pem)
    }

    pub fn new_public_key(&self, private_key: &TardisCryptoRsaPrivateKey) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_private_key(private_key)
    }

    pub fn new_public_key_from_public_key_pem(&self, public_key_pem: &str) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_public_key_pem(public_key_pem)
    }

    pub fn new_public_key_from_private_key_pem(&self, private_key_pem: &str) -> TardisResult<TardisCryptoRsaPublicKey> {
        TardisCryptoRsaPublicKey::from_private_key_pem(private_key_pem)
    }
}

impl TardisCryptoRsaPrivateKey {
    pub fn new(bits: usize) -> TardisResult<Self> {
        let mut rand = rand::rngs::OsRng;
        Ok(TardisCryptoRsaPrivateKey {
            pri_key: rsa::RsaPrivateKey::new(&mut rand, bits)?,
        })
    }

    pub fn from_private_key_pem(private_key_pem: &str) -> TardisResult<Self> {
        Ok(TardisCryptoRsaPrivateKey {
            pri_key: rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?,
        })
    }

    pub fn to_private_key_pem(&self) -> TardisResult<String> {
        Ok(self.pri_key.to_pkcs8_pem()?.to_string())
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let mut rand = rand::rngs::OsRng;
        let encrypted_data = self.pri_key.encrypt(&mut rand, rsa::PaddingScheme::PKCS1v15Encrypt, data.as_bytes())?;
        Ok(base64::encode(encrypted_data))
    }

    pub fn decrypt(&self, encrypted_data: &str) -> TardisResult<String> {
        let encrypted_data = base64::decode(encrypted_data)?;
        let data = self.pri_key.decrypt(rsa::PaddingScheme::PKCS1v15Encrypt, encrypted_data.as_slice())?;
        Ok(String::from_utf8(data)?)
    }

    pub fn sign(&self, data: &str) -> TardisResult<String> {
        let signed_data = self.pri_key.sign(rsa::PaddingScheme::PKCS1v15Sign { hash: None }, TardisFuns::crypto.digest.sha256(data)?.as_bytes())?;
        Ok(base64::encode(signed_data))
    }

    pub fn verify(&self, data: &str, signed_data: &str) -> TardisResult<bool> {
        let signed_data = base64::decode(signed_data)?;
        let result = self.pri_key.verify(
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

impl TardisCryptoRsaPublicKey {
    pub fn from_private_key(private_key: &TardisCryptoRsaPrivateKey) -> TardisResult<Self> {
        let public_key = rsa::RsaPublicKey::from(&private_key.pri_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_private_key_pem(private_key_pem: &str) -> TardisResult<Self> {
        let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
        let public_key = rsa::RsaPublicKey::from(private_key);
        Ok(TardisCryptoRsaPublicKey { pub_key: public_key })
    }

    pub fn from_public_key_pem(public_key_pem: &str) -> TardisResult<Self> {
        Ok(TardisCryptoRsaPublicKey {
            pub_key: rsa::RsaPublicKey::from_public_key_pem(public_key_pem)?,
        })
    }

    pub fn to_public_key_pem(&self) -> TardisResult<String> {
        Ok(self.pub_key.to_public_key_pem()?)
    }

    pub fn encrypt(&self, data: &str) -> TardisResult<String> {
        let mut rand = rand::rngs::OsRng;
        let encrypted_data = self.pub_key.encrypt(&mut rand, rsa::PaddingScheme::PKCS1v15Encrypt, data.as_bytes())?;
        Ok(base64::encode(encrypted_data))
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
        Ok(format!("tk{}", TardisFuns::field.uuid()))
    }

    pub fn generate_ak(&self) -> TardisResult<String> {
        Ok(format!("ak{}", TardisFuns::field.uuid()))
    }

    pub fn generate_sk(&self, ak: &str) -> TardisResult<String> {
        let sk = TardisFuns::crypto.digest.sha1(format!("{}{}", ak, TardisFuns::field.uuid()).as_str());
        match sk {
            Ok(sk) => Ok(sk),
            Err(e) => Err(e),
        }
    }
}

#[cfg(feature = "crypto")]
impl From<crypto::symmetriccipher::SymmetricCipherError> for TardisError {
    fn from(error: crypto::symmetriccipher::SymmetricCipherError) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] AES crypto error, {:?}", error))
    }
}

#[cfg(feature = "crypto")]
impl From<rsa::errors::Error> for TardisError {
    fn from(error: rsa::errors::Error) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] RSA crypto error, {}", error))
    }
}

#[cfg(feature = "crypto")]
impl From<pkcs8::Error> for TardisError {
    fn from(error: pkcs8::Error) -> Self {
        TardisError::FormatError(format!("[Tardis.Crypto] RSA crypto error, {}", error))
    }
}
