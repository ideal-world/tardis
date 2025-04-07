use rand::RngCore;

use crate::{basic::result::TardisResult, TardisFuns};
use paste::paste;
pub struct TardisCryptoKey;

macro_rules! gen_rand_n_hex {
    {$($N: literal),*} => {
        $(
            paste! {
                #[inline]
                #[doc = "generate a random hex string with length of " $N "."]
                pub fn [<rand_ $N _hex>](&self) -> String {
                    self.rand_n_hex::<$N>()
                }
            }
        )*
    };
}
macro_rules! gen_rand_n_bytes {
    {$($N: literal),*} => {
        $(
            paste! {
                #[inline]
                #[doc = "generate random " $N " bytes."]
                pub fn [<rand_ $N _bytes>](&self) -> [u8; $N] {
                    self.rand_n_bytes::<$N>()
                }
            }
        )*
    };
}
impl TardisCryptoKey {
    /// generate a random hex string with length of N
    pub fn rand_n_hex<const N: usize>(&self) -> String {
        let mut key = vec![0; N / 2];
        rand::rngs::ThreadRng::default().fill_bytes(&mut key);
        hex::encode(key)
    }

    /// generate random N bytes
    pub fn rand_n_bytes<const N: usize>(&self) -> [u8; N] {
        let mut key = [0; N];
        rand::rngs::ThreadRng::default().fill_bytes(&mut key);
        key
    }
    gen_rand_n_hex! {8, 16, 32, 64, 128, 256}
    gen_rand_n_bytes! {8, 16, 32, 64, 128, 256}
    /// generate a token with prefix "tk" followed by a random nanoid
    pub fn generate_token(&self) -> TardisResult<String> {
        Ok(format!("tk{}", TardisFuns::field.nanoid()))
    }

    /// generate a access key with prefix "ak" followed by a random nanoid
    pub fn generate_ak(&self) -> TardisResult<String> {
        Ok(format!("ak{}", TardisFuns::field.nanoid()))
    }

    /// generate a secret key with by ak and random seed
    pub fn generate_sk(&self, ak: &str) -> TardisResult<String> {
        let sk = TardisFuns::crypto.digest.sha1(format!("{}{}", ak, TardisFuns::field.nanoid()).as_str());
        match sk {
            Ok(sk) => Ok(sk),
            Err(error) => Err(error),
        }
    }
}
