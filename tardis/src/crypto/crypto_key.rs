use rand::RngCore;

use crate::{basic::result::TardisResult, TardisFuns};
use paste::paste;
pub struct TardisCryptoKey;

macro_rules! gen_rand_n_hex {
    {$($N: literal),*} => {
        $(
            paste! {
                #[inline]
                pub fn [<rand_ $N _hex>](&self) -> TardisResult<String> {
                    self.rand_n_hex::<$N>()
                }
            }
        )*
    };
}

impl TardisCryptoKey {
    pub fn rand_n_hex<const N: usize>(&self) -> TardisResult<String> {
        let mut key = vec![0; N / 2];
        rand::rngs::OsRng.fill_bytes(&mut key);
        Ok(hex::encode(key))
    }
    gen_rand_n_hex! {8, 16, 32, 64, 128, 256}

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
