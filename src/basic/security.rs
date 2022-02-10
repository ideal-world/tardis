use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha256, Sha512};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::basic::security::Algorithm::SHA1;
use crate::TardisFuns;

pub struct TardisSecurity {
    pub base64: TardisSecurityBase64,
    pub key: TardisSecurityKey,
}

pub struct TardisSecurityBase64;
pub struct TardisSecurityKey;

pub enum Algorithm {
    MD5,
    SHA1,
    SHA256,
    SHA512,
    HmacSha1,
    HmacSha265,
    HmacSha512,
}

impl TardisSecurity {
    pub fn digest(&self, str: &str, key: Option<&str>, algorithm: Algorithm) -> TardisResult<String> {
        match algorithm {
            Algorithm::SHA1 => {
                let mut sha1 = Sha1::new();
                sha1.input_str(str);
                Ok(sha1.result_str())
            }
            Algorithm::SHA256 => {
                let mut sha265 = Sha256::new();
                sha265.input_str(str);
                Ok(sha265.result_str())
            }
            Algorithm::SHA512 => {
                let mut sha512 = Sha512::new();
                sha512.input_str(str);
                Ok(sha512.result_str())
            }
            Algorithm::MD5 => {
                let mut md5 = Md5::new();
                md5.input_str(str);
                Ok(md5.result_str())
            }
            Algorithm::HmacSha1 => match key {
                Some(key) => {
                    let mut hmac = Hmac::new(Sha1::new(), key.as_bytes());
                    hmac.input(str.as_bytes());
                    Ok(base64::encode(hmac.result().code()))
                }
                None => Err(TardisError::BadRequest("[Tardis.Security] key is required for hmacsha1".to_string())),
            },
            Algorithm::HmacSha265 => match key {
                Some(key) => {
                    let mut hmac = Hmac::new(Sha256::new(), key.as_bytes());
                    hmac.input(str.as_bytes());
                    Ok(base64::encode(hmac.result().code()))
                }
                None => Err(TardisError::BadRequest("[Tardis.Security] key is required for hmacsha256".to_string())),
            },
            Algorithm::HmacSha512 => match key {
                Some(key) => {
                    let mut hmac = Hmac::new(Sha512::new(), key.as_bytes());
                    hmac.input(str.as_bytes());
                    Ok(base64::encode(hmac.result().code()))
                }
                None => Err(TardisError::BadRequest("[Tardis.Security] key is required for hmacsha512".to_string())),
            },
        }
    }
}

impl TardisSecurityBase64 {
    pub fn decode(&self, str: &str) -> TardisResult<String> {
        match base64::decode(str) {
            Ok(result) => Ok(String::from_utf8(result).expect("Vec[] to String error")),
            Err(e) => Err(TardisError::FormatError(e.to_string())),
        }
    }

    pub fn encode(&self, str: &str) -> String {
        base64::encode(str)
    }
}

impl TardisSecurityKey {
    pub fn generate_token(&self) -> TardisResult<String> {
        Ok(format!("tk{}", TardisFuns::field.uuid()))
    }

    pub fn generate_ak(&self) -> TardisResult<String> {
        Ok(format!("ak{}", TardisFuns::field.uuid()))
    }

    pub fn generate_sk(&self, ak: &str) -> TardisResult<String> {
        let sk = TardisFuns::security.digest(format!("{}{}", ak, TardisFuns::field.uuid()).as_str(), None, SHA1);
        match sk {
            Ok(sk) => Ok(sk),
            Err(e) => Err(e),
        }
    }
}
