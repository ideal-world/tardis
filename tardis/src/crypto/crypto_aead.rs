use aead::generic_array::GenericArray;
use aead::{Aead, AeadCore, KeyInit, Payload};
use cipher::BlockDecryptMut;
use cipher::{block_padding::Pkcs7, BlockCipher, BlockEncryptMut, KeyIvInit};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

use rand::rngs::ThreadRng;

/// # TardisCryptoAead
/// Aead (Authenticated Encryption with Associated Data）
///
/// This part includes aead encryption and decryption, and cbc/ecb encryption and decryption.
pub struct TardisCryptoAead {}

impl TardisCryptoAead {
    /// Encrypy with cbc,
    /// `A` could be any algorithem implemented `BlockEncryptMut + BlockCipher`, a typical one would be `Aes128`.
    ///
    /// # **Warning**
    /// cbc mode is not recommended, it is not safe enough.
    pub fn encrypt_cbc<A: BlockEncryptMut + BlockCipher>(&self, message: impl AsRef<[u8]>, iv: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<Vec<u8>>
    where
        cbc::Encryptor<A>: KeyIvInit + BlockEncryptMut,
    {
        let iv = GenericArray::from_slice(iv.as_ref());
        let message = message.as_ref();
        let key = GenericArray::from_slice(key.as_ref());
        let encryptor = <cbc::Encryptor<A> as KeyIvInit>::new(key, iv);
        let ct = <cbc::Encryptor<A> as cipher::BlockEncryptMut>::encrypt_padded_vec_mut::<Pkcs7>(encryptor, message);
        // let ct = encryptor.encrypt_padded_vec_mut::<Pkcs7>(message);
        Ok(ct)
    }

    /// Encrypy with ecb,
    /// `A` could be any algorithem implemented `BlockEncryptMut + BlockCipher`, a typical one would be `Aes128`.
    /// # **Warning**
    /// cbc mode is not recommended, it is not safe enough.
    pub fn encrypt_ecb<A: BlockEncryptMut + BlockCipher>(&self, message: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<Vec<u8>>
    where
        ecb::Encryptor<A>: KeyInit + BlockEncryptMut,
    {
        let message = message.as_ref();
        let key = GenericArray::from_slice(key.as_ref());
        let encryptor = <ecb::Encryptor<A> as KeyInit>::new(key);
        let ct = encryptor.encrypt_padded_vec_mut::<Pkcs7>(message);
        Ok(ct)
    }

    /// Decrypy with cbc,
    /// `A` could be any algorithem implemented `BlockEncryptMut + BlockCipher`, a typical one would be `Aes128`.
    pub fn decrypt_cbc<A: BlockDecryptMut + BlockCipher>(&self, message: impl AsRef<[u8]>, iv: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<Vec<u8>>
    where
        cbc::Decryptor<A>: KeyIvInit + BlockDecryptMut,
    {
        let iv = GenericArray::from_slice(iv.as_ref());
        let message = message.as_ref();
        let key = GenericArray::from_slice(key.as_ref());
        let decryptor = <cbc::Decryptor<A> as KeyIvInit>::new(key, iv);
        let pt = decryptor.decrypt_padded_vec_mut::<Pkcs7>(message).map_err(|e| TardisError::internal_error(&e.to_string(), "406-tardis-crypto-aead-decrypt-failed"))?;
        Ok(pt)
    }

    /// Decrypy with ecb,
    /// `A` could be any algorithem implemented `BlockEncryptMut + BlockCipher`, a typical one would be `Aes128`.
    pub fn decrypt_ecb<A: BlockDecryptMut + BlockCipher>(&self, message: impl AsRef<[u8]>, key: impl AsRef<[u8]>) -> TardisResult<Vec<u8>>
    where
        ecb::Decryptor<A>: KeyInit + BlockDecryptMut,
    {
        let key = GenericArray::from_slice(key.as_ref());
        let decryptor = <ecb::Decryptor<A> as KeyInit>::new(key);
        let pt = decryptor.decrypt_padded_vec_mut::<Pkcs7>(message.as_ref()).map_err(|e| TardisError::internal_error(&e.to_string(), "406-tardis-crypto-aead-decrypt-failed"))?;
        Ok(pt)
    }

    /// Encrypt with aead algorithm,
    /// `A` could be any algorithem implemented `Aead + KeyInit`, a typical one would be `Aes256Gcm`.
    pub fn encrypt<A: Aead + KeyInit>(&self, key: impl AsRef<[u8]>, aad: impl AsRef<[u8]>, nonce: impl AsRef<[u8]>, message: impl AsRef<[u8]>) -> TardisResult<(Vec<u8>, Vec<u8>)> {
        let key = GenericArray::from_slice(key.as_ref());
        let nonce = GenericArray::from_slice(nonce.as_ref());
        let payload = Payload {
            msg: message.as_ref(),
            aad: aad.as_ref(),
        };
        let cipher = A::new(key);
        let ciphertext = cipher.encrypt(nonce, payload).map_err(|e| TardisError::internal_error(&e.to_string(), "406-tardis-crypto-aead-encrypt-failed"))?;
        Ok((ciphertext, nonce.to_vec()))
    }

    /// Decrypt with aead algorithm,
    pub fn decrypt<A: Aead + KeyInit>(&self, key: impl AsRef<[u8]>, aad: impl AsRef<[u8]>, nonce: impl AsRef<[u8]>, message: impl AsRef<[u8]>) -> TardisResult<Vec<u8>> {
        let key = GenericArray::from_slice(key.as_ref());
        let nonce = GenericArray::from_slice(nonce.as_ref());
        let payload = Payload {
            msg: message.as_ref(),
            aad: aad.as_ref(),
        };
        let cipher = A::new(key);
        let plaintext = cipher.decrypt(nonce, payload).map_err(|e| TardisError::internal_error(&e.to_string(), "406-tardis-crypto-aead-encrypt-failed"))?;
        Ok(plaintext)
    }

    /// Generate a random nonce for aead algorithm.
    pub fn random_nonce<A>(&self) -> Vec<u8>
    where
        A: AeadCore,
    {
        let nonce = A::generate_nonce(ThreadRng::default());
        nonce.to_vec()
    }
}

pub mod algorithm {
    pub use aes::{Aes128, Aes192, Aes256};
    pub use aes_gcm::{Aes128Gcm, Aes256Gcm};
    pub use aes_gcm_siv::{Aes128GcmSiv, Aes256GcmSiv};
    pub use aes_siv::{Aes128SivAead, Aes256SivAead};
}
