use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new_from_slice(key).expect("validated AES key"),
        }
    }

    pub fn encrypt_json<T: Serialize>(&self, value: &T) -> Result<String, String> {
        let plaintext = serde_json::to_vec(value).map_err(|_| "failed to encode secret data")?;
        let mut nonce_bytes = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
            .map_err(|_| "failed to encrypt secret data")?;
        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);
        Ok(STANDARD.encode(payload))
    }

    pub fn decrypt_json<T: DeserializeOwned>(&self, value: &str) -> Result<T, String> {
        let payload = STANDARD
            .decode(value)
            .map_err(|_| "encrypted secret data is invalid")?;
        if payload.len() < 13 {
            return Err("encrypted secret data is truncated".to_string());
        }
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(&payload[..12]), &payload[12..])
            .map_err(|_| "failed to decrypt secret data")?;
        serde_json::from_slice(&plaintext)
            .map_err(|_| "decrypted secret data is invalid".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Crypto;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Secret {
        token: String,
    }

    #[test]
    fn encrypted_json_round_trips_without_plaintext() {
        let crypto = Crypto::new(&[3_u8; 32]);
        let source = Secret {
            token: "very-secret-token".to_string(),
        };
        let encrypted = crypto.encrypt_json(&source).unwrap();
        assert!(!encrypted.contains("very-secret-token"));
        assert_eq!(crypto.decrypt_json::<Secret>(&encrypted).unwrap(), source);
    }
}
