// use crypto::mac::Mac;
use crate::{
    basic::{error::TardisError, result::TardisResult},
    utils::mapper::Mapper,
};
use algorithm::*;
use digest::KeyInit;

use output::*;
pub struct TardisCryptoDigest;

/// algorithms for digest
pub mod algorithm {
    pub use digest::Digest;
    pub use hmac::{Hmac, Mac};
    pub use md5::Md5;
    pub use sha1::Sha1;
    pub use sha2::{OidSha224, OidSha256, OidSha384, OidSha512, Sha224, Sha256, Sha384, Sha512};
    pub use sm3::Sm3;
    pub type HmacSha1 = Hmac<Sha1>;
    pub type HmacSha256 = Hmac<Sha256>;
    pub type HmacSha512 = Hmac<Sha512>;
}

pub mod output {
    use std::marker::PhantomData;

    pub use digest::Output;

    use crate::utils::mapper::Mapper;

    /// Mapper digest output into hexcode
    #[derive(Default, Debug)]
    pub struct HexCodeMapper<A: digest::Digest>(PhantomData<A>);
    impl<A: digest::Digest> Mapper<digest::Output<A>> for HexCodeMapper<A> {
        type Output = String;
        fn map(raw_output: digest::Output<A>) -> Self::Output {
            hex::encode(raw_output)
        }
    }

    /// Mapper digest output into bytes
    #[derive(Default, Debug)]
    pub struct BytesMapper<A: digest::Digest>(PhantomData<A>);
    impl<A: digest::Digest> Mapper<digest::Output<A>> for BytesMapper<A> {
        type Output = Vec<u8>;
        fn map(raw_output: digest::Output<A>) -> Self::Output {
            raw_output.to_vec()
        }
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
    pub fn sha1(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest::<sha1::Sha1>(data)
    }

    pub fn sha256(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest::<sha2::Sha256>(data)
    }

    pub fn sha512(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest::<sha2::Sha512>(data)
    }

    pub fn md5(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest::<md5::Md5>(data)
    }

    pub fn sm3(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest::<sm3::Sm3>(data)
    }

    pub fn hmac_sha1(&self, data: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_hmac::<HmacSha1>(data, key)
    }

    pub fn hmac_sha256(&self, data: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_hmac::<HmacSha256>(data, key)
    }

    pub fn hmac_sha512(&self, data: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_hmac::<HmacSha512>(data, key)
    }

    /// Digest the data, and map the output into hexcode by default.
    ///
    /// # Examples
    /// ```ignore
    /// TardisCryptoDigest.digest::<algorithm::Sha1>("测试").unwrap();
    /// ```
    pub fn digest<A: digest::Digest>(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_hex::<A>(data)
    }

    /// Digest the data, and map the output into hexcode.
    pub fn digest_hex<A: digest::Digest>(&self, data: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_as::<A, HexCodeMapper<A>>(data)
    }

    /// Digest the data, and map the output into `Vec<u8>`.
    pub fn digest_bytes<A: digest::Digest>(&self, data: impl AsRef<[u8]>) -> TardisResult<Vec<u8>> {
        self.digest_as::<A, BytesMapper<A>>(data)
    }

    /// Digest the data, and map the output into a specific type which determined by `M`.
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::crypto::crypto_digest::{TardisCryptoDigest, algorithm::Sha1, output::HexCodeMapper};
    /// let hexcode = TardisCryptoDigest.digest_as::<Sha1, HexCodeMapper<Sha1>>("测试").unwrap();
    /// ```
    pub fn digest_as<A: digest::Digest, M: Mapper<digest::Output<A>>>(&self, data: impl AsRef<[u8]>) -> TardisResult<M::Output> {
        self.digest_iter_as::<A, M, _>(Some(data))
    }

    /// Digest a sequence of data, and map the output into a specific type which determined by `M`.
    pub fn digest_iter_as<A: digest::Digest, M: Mapper<digest::Output<A>>, T: AsRef<[u8]>>(&self, data_iter: impl IntoIterator<Item = T>) -> TardisResult<M::Output> {
        self.digest_iter_raw::<A, T>(data_iter).map(M::map)
    }

    /// Digest a sequence of data.
    ///
    /// Get the raw digest output from Digest trait, the type is determined by althogrim itself (most time it's a GenericArray).
    pub fn digest_iter_raw<A: digest::Digest, T: AsRef<[u8]>>(&self, data_iter: impl IntoIterator<Item = T>) -> TardisResult<Output<A>> {
        let mut hasher = A::new();
        for data in data_iter {
            hasher.update(data);
        }
        let out = hasher.finalize();
        Ok(out)
    }

    /// Digest the data, and map the output into hexcode by default.
    pub fn digest_hmac<A: Mac + KeyInit>(&self, data: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<String> {
        self.digest_hmac_raw::<A>(data, key).map(hex::encode)
    }

    /// Digest the data
    pub fn digest_hmac_raw<A: Mac + KeyInit>(&self, data: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<Vec<u8>> {
        let mut hmac = <A as Mac>::new_from_slice(key.as_ref()).map_err(|_| TardisError::internal_error("hmac key with invalid length", "406-tardis-crypto-hmac-key-invalid"))?;
        hmac.update(data.as_ref());
        Ok(hmac.finalize().into_bytes().to_vec())
    }
}
