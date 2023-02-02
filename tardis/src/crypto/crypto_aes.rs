use crypto::buffer::{ReadBuffer, WriteBuffer};

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

pub struct TardisCryptoAes;

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
    pub fn encrypt_ecb(&self, data: &str, hex_key: &str) -> TardisResult<String> {
        self.encrypt(data, hex_key, "", false)
    }

    pub fn encrypt_cbc(&self, data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        self.encrypt(data, hex_key, hex_iv, true)
    }

    fn encrypt(&self, data: &str, hex_key: &str, hex_iv: &str, cbc_mode: bool) -> TardisResult<String> {
        let key_size = match hex_key.len() {
            16 => crypto::aes::KeySize::KeySize128,
            24 => crypto::aes::KeySize::KeySize192,
            32 => crypto::aes::KeySize::KeySize256,
            _ => {
                return Err(TardisError::format_error(
                    "[Tardis.Crypto] AES error, invalid key size",
                    "406-tardis-crypto-aes-key-invalid",
                ))
            }
        };

        let mut encryptor = if cbc_mode {
            crypto::aes::cbc_encryptor(key_size, hex_key.as_bytes(), hex_iv.as_bytes(), crypto::blockmodes::PkcsPadding)
        } else {
            crypto::aes::ecb_encryptor(key_size, hex_key.as_bytes(), crypto::blockmodes::PkcsPadding)
        };

        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = crypto::buffer::RefReadBuffer::new(data.as_bytes());
        let mut buffer = [0; 4096];
        let mut write_buffer = crypto::buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true)?;
            final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());
            match result {
                crypto::buffer::BufferResult::BufferUnderflow => break,
                crypto::buffer::BufferResult::BufferOverflow => {}
            }
        }
        Ok(hex::encode(final_result))
    }

    pub fn decrypt_ecb(&self, encrypted_data: &str, hex_key: &str) -> TardisResult<String> {
        self.decrypt(encrypted_data, hex_key, "", false)
    }

    pub fn decrypt_cbc(&self, encrypted_data: &str, hex_key: &str, hex_iv: &str) -> TardisResult<String> {
        self.decrypt(encrypted_data, hex_key, hex_iv, true)
    }

    fn decrypt(&self, encrypted_data: &str, hex_key: &str, hex_iv: &str, cbc_mode: bool) -> TardisResult<String> {
        let key_size = match hex_key.len() {
            16 => crypto::aes::KeySize::KeySize128,
            24 => crypto::aes::KeySize::KeySize192,
            32 => crypto::aes::KeySize::KeySize256,
            _ => {
                return Err(TardisError::format_error(
                    "[Tardis.Crypto] AES error, invalid key size",
                    "406-tardis-crypto-aes-key-invalid",
                ))
            }
        };

        let encrypted_data = hex::decode(encrypted_data)?;

        let mut decryptor = if cbc_mode {
            crypto::aes::cbc_decryptor(key_size, hex_key.as_bytes(), hex_iv.as_bytes(), crypto::blockmodes::PkcsPadding)
        } else {
            crypto::aes::ecb_decryptor(key_size, hex_key.as_bytes(), crypto::blockmodes::PkcsPadding)
        };

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

impl From<crypto::symmetriccipher::SymmetricCipherError> for TardisError {
    fn from(error: crypto::symmetriccipher::SymmetricCipherError) -> Self {
        TardisError::format_error(&format!("[Tardis.Crypto] AES crypto error, {error:?}"), "406-tardis-crypto-aes-error")
    }
}
