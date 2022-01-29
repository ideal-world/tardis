use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha256, Sha512};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::TardisFuns;

pub struct TardisSecurity {
    pub base64: TardisSecurityBase64,
    pub key: TardisSecurityKey,
}

pub struct TardisSecurityBase64;
pub struct TardisSecurityKey;

impl TardisSecurity {
    pub fn digest(&self, str: &str, key: Option<&str>, algorithm: &str) -> String {
        match algorithm.to_lowercase().as_str() {
            "sha1" => {
                let mut sha1 = Sha1::new();
                sha1.input_str(str);
                sha1.result_str()
            }
            "sha256" => {
                let mut sha265 = Sha256::new();
                sha265.input_str(str);
                sha265.result_str()
            }
            "sha512" => {
                let mut sha512 = Sha512::new();
                sha512.input_str(str);
                sha512.result_str()
            }
            "md5" => {
                let mut md5 = Md5::new();
                md5.input_str(str);
                md5.result_str()
            }
            "hmacsha1" => {
                let mut hmac = Hmac::new(Sha1::new(), key.unwrap().as_bytes());
                hmac.input(str.as_bytes());
                String::from_utf8(hmac.result().code().to_vec()).expect("Abstract algorithm conversion error")
            }
            "hmacsha256" => {
                let mut hmac = Hmac::new(Sha256::new(), key.unwrap().as_bytes());
                hmac.input(str.as_bytes());
                String::from_utf8(hmac.result().code().to_vec()).expect("Abstract algorithm conversion error")
            }
            "hmacsha512" => {
                let mut hmac = Hmac::new(Sha512::new(), key.unwrap().as_bytes());
                hmac.input(str.as_bytes());
                String::from_utf8(hmac.result().code().to_vec()).expect("Abstract algorithm conversion error")
            }
            _ => panic!("[Tardis.Security] Digest algorithm [{}] doesn't support", algorithm),
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
    pub fn generate_token(&self) -> String {
        format!("tk{}", TardisFuns::field.uuid())
    }

    pub fn generate_ak(&self) -> String {
        format!("ak{}", TardisFuns::field.uuid())
    }

    pub fn generate_sk(&self, ak: &str) -> String {
        let sk = TardisFuns::security.digest(format!("{}{}", ak, TardisFuns::field.uuid()).as_str(), None, "SHA1");
        format!("sk{}", sk)
    }
}
