use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, AeadCore, Nonce,
};
use tesseras_core::enums::EncryptionContext;

use crate::CryptoError;

/// Nonce (12 bytes) + ciphertext (variable length).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedBlob {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

/// AES-256-GCM symmetric encryption with random nonce.
pub struct Aes256GcmEncryptor;

impl Aes256GcmEncryptor {
    pub fn encrypt(
        content: &[u8],
        key: &[u8; 32],
        context: &EncryptionContext,
    ) -> Result<EncryptedBlob, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| CryptoError::EncryptFailed(e.to_string()))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let aad = context.to_aad_bytes();
        let ciphertext = cipher
            .encrypt(
                &nonce,
                aes_gcm::aead::Payload {
                    msg: content,
                    aad: &aad,
                },
            )
            .map_err(|e| CryptoError::EncryptFailed(e.to_string()))?;
        Ok(EncryptedBlob {
            nonce: nonce.into(),
            ciphertext,
        })
    }

    pub fn decrypt(
        blob: &EncryptedBlob,
        key: &[u8; 32],
        context: &EncryptionContext,
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::DecryptFailed)?;
        let nonce = Nonce::from_slice(&blob.nonce);
        let aad = context.to_aad_bytes();
        cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: &blob.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| CryptoError::DecryptFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::ContentHash;

    fn test_key() -> [u8; 32] {
        [0x42u8; 32]
    }

    fn test_private_ctx() -> EncryptionContext {
        EncryptionContext::Private {
            content_hash: ContentHash::new([0xaa; 32]),
        }
    }

    fn test_sealed_ctx() -> EncryptionContext {
        use chrono::TimeZone;
        EncryptionContext::Sealed {
            content_hash: ContentHash::new([0xbb; 32]),
            open_after: chrono::Utc.with_ymd_and_hms(2050, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn aes256gcm_roundtrip() {
        let plaintext = b"hello tesseras";
        let key = test_key();
        let ctx = test_private_ctx();

        let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
        let recovered = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn aes256gcm_wrong_key() {
        let plaintext = b"secret data";
        let key = test_key();
        let wrong_key = [0x99u8; 32];
        let ctx = test_private_ctx();

        let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
        let result = Aes256GcmEncryptor::decrypt(&blob, &wrong_key, &ctx);
        assert!(matches!(result, Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn aes256gcm_tampered_ciphertext() {
        let plaintext = b"important data";
        let key = test_key();
        let ctx = test_private_ctx();

        let mut blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
        if let Some(byte) = blob.ciphertext.first_mut() {
            *byte ^= 0xff;
        }
        let result = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx);
        assert!(matches!(result, Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn aes256gcm_wrong_aad() {
        use chrono::TimeZone;
        let plaintext = b"sealed content";
        let key = test_key();
        let ctx_2050 = EncryptionContext::Sealed {
            content_hash: ContentHash::new([0xbb; 32]),
            open_after: chrono::Utc.with_ymd_and_hms(2050, 1, 1, 0, 0, 0).unwrap(),
        };
        let ctx_2025 = EncryptionContext::Sealed {
            content_hash: ContentHash::new([0xbb; 32]),
            open_after: chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        };

        let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx_2050).unwrap();
        // Trying to decrypt with a different open_after fails — prevents timestamp tampering
        let result = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx_2025);
        assert!(matches!(result, Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn aes256gcm_private_vs_sealed_context() {
        let plaintext = b"same content";
        let key = test_key();
        let private_ctx = test_private_ctx();
        let sealed_ctx = test_sealed_ctx();

        let blob_private = Aes256GcmEncryptor::encrypt(plaintext, &key, &private_ctx).unwrap();
        let blob_sealed = Aes256GcmEncryptor::encrypt(plaintext, &key, &sealed_ctx).unwrap();

        // Different contexts produce different ciphertext (different AAD + different nonce)
        assert_ne!(blob_private.ciphertext, blob_sealed.ciphertext);

        // Cross-decryption fails
        let result = Aes256GcmEncryptor::decrypt(&blob_private, &key, &sealed_ctx);
        assert!(matches!(result, Err(CryptoError::DecryptFailed)));
        let result = Aes256GcmEncryptor::decrypt(&blob_sealed, &key, &private_ctx);
        assert!(matches!(result, Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn aes256gcm_unique_nonce_per_call() {
        let key = test_key();
        let plaintext = b"same content";
        let ctx = test_private_ctx();

        let blob1 = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
        let blob2 = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
        assert_ne!(blob1.nonce, blob2.nonce);
        assert_ne!(blob1.ciphertext, blob2.ciphertext);
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use std::collections::HashSet;

        proptest! {
            #[test]
            fn prop_encrypt_decrypt_any_payload(
                plaintext in proptest::collection::vec(any::<u8>(), 0..10000),
            ) {
                let key = [0x42u8; 32];
                let ctx = EncryptionContext::Private {
                    content_hash: ContentHash::new([0xaa; 32]),
                };
                let blob = Aes256GcmEncryptor::encrypt(&plaintext, &key, &ctx).unwrap();
                let recovered = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx).unwrap();
                prop_assert_eq!(recovered, plaintext);
            }

            #[test]
            fn prop_nonce_never_repeats(
                plaintext in proptest::collection::vec(any::<u8>(), 1..1000),
            ) {
                let key = [0x42u8; 32];
                let ctx = EncryptionContext::Private {
                    content_hash: ContentHash::new([0xaa; 32]),
                };
                let nonces: Vec<[u8; 12]> = (0..100)
                    .map(|_| Aes256GcmEncryptor::encrypt(&plaintext, &key, &ctx).unwrap().nonce)
                    .collect();
                let unique: HashSet<[u8; 12]> = nonces.iter().copied().collect();
                prop_assert_eq!(unique.len(), 100);
            }
        }
    }
}
