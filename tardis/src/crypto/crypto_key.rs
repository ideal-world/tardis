use rand::RngCore;

use crate::{basic::result::TardisResult, TardisFuns};

pub struct TardisCryptoKey;

impl TardisCryptoKey {
    pub fn rand_8_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 4] = [0; 4];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_16_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 8] = [0; 8];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_32_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 16] = [0; 16];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_64_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 32] = [0; 32];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_128_hex(&self) -> TardisResult<String> {
        let mut key: [u8; 64] = [0; 64];
        rand::rngs::OsRng::default().fill_bytes(&mut key);
        Ok(hex::encode(key))
    }

    pub fn rand_256_hex(&self) -> TardisResult<String> {
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
            Err(error) => Err(error),
        }
    }
}
