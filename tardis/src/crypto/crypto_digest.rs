use crypto::mac::Mac;

use crate::basic::result::TardisResult;
pub struct TardisCryptoDigest;

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

    #[cfg(feature = "crypto-with-sm")]
    pub fn sm3(&self, data: &str) -> TardisResult<String> {
        use libsm::sm3::hash::Sm3Hash;

        Ok(hex::encode(Sm3Hash::new(data.as_bytes()).get_hash()))
    }

    fn digest<A: crypto::digest::Digest>(&self, data: &str, mut algorithm: A) -> TardisResult<String> {
        algorithm.input_str(data);
        Ok(algorithm.result_str())
    }

    fn digest_hmac<A: crypto::digest::Digest>(&self, data: &str, key: &str, algorithm: A) -> TardisResult<String> {
        let mut hmac = crypto::hmac::Hmac::new(algorithm, key.as_bytes());
        hmac.input(data.as_bytes());
        Ok(hex::encode(hmac.result().code()))
    }
}
