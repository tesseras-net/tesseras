use tesseras_core::ContentHash;

use crate::CryptoError;
use crate::dual::{DualKeyPair, DualPublicKeys, DualSignature, sign_manifest, verify_manifest};
use crate::kem::{ContentKey, HybridCiphertext, HybridEncryptionPublic, HybridKem, HybridKeyPair};

/// Content key encrypted to the owner's hybrid public key.
/// Lives in `manifest.json` per-tessera, not in identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedKeyEnvelope {
    pub hybrid_ciphertext: HybridCiphertext,
}

/// Standalone signed artifact for publishing a sealed tessera's content key
/// after `open_after` has passed. Manifest stays immutable.
#[derive(Debug, Clone)]
pub struct KeyPublication {
    pub tessera_hash: ContentHash,
    pub content_key: ContentKey,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub signature: DualSignature,
}

impl SealedKeyEnvelope {
    /// Encrypt a content key to the owner's public key.
    pub fn seal(
        content_key: &ContentKey,
        owner_public: &HybridEncryptionPublic,
    ) -> Result<Self, CryptoError> {
        let (transport_key, ciphertext) = HybridKem::encapsulate(owner_public)?;

        // XOR content_key with transport_key (both 32 bytes)
        let mut wrapped = [0u8; 32];
        for i in 0..32 {
            wrapped[i] = content_key[i] ^ transport_key[i];
        }

        // Store wrapped key appended to the KEM ciphertext
        let mut mlkem_ct_with_wrapped = ciphertext.mlkem_ciphertext;
        mlkem_ct_with_wrapped.extend_from_slice(&wrapped);

        Ok(SealedKeyEnvelope {
            hybrid_ciphertext: HybridCiphertext {
                x25519_ephemeral: ciphertext.x25519_ephemeral,
                mlkem_ciphertext: mlkem_ct_with_wrapped,
            },
        })
    }

    /// Decrypt the content key using the owner's private key.
    pub fn unseal(&self, owner_keypair: &HybridKeyPair) -> Result<ContentKey, CryptoError> {
        // Split the mlkem ciphertext: last 32 bytes are the wrapped content key
        let ct_bytes = &self.hybrid_ciphertext.mlkem_ciphertext;
        if ct_bytes.len() < 32 {
            return Err(CryptoError::DecryptFailed);
        }
        let (mlkem_ct_bytes, wrapped) = ct_bytes.split_at(ct_bytes.len() - 32);

        let inner_ciphertext = HybridCiphertext {
            x25519_ephemeral: self.hybrid_ciphertext.x25519_ephemeral,
            mlkem_ciphertext: mlkem_ct_bytes.to_vec(),
        };

        let transport_key = HybridKem::decapsulate(&inner_ciphertext, owner_keypair)?;

        // XOR to recover content key
        let mut content_key = [0u8; 32];
        for i in 0..32 {
            content_key[i] = wrapped[i] ^ transport_key[i];
        }

        Ok(content_key)
    }
}

impl KeyPublication {
    /// Create a signed key publication for a sealed tessera whose open_after has passed.
    pub fn create(
        tessera_hash: ContentHash,
        content_key: ContentKey,
        signing_keys: &DualKeyPair,
    ) -> Self {
        let published_at = chrono::Utc::now();
        let message = Self::signing_message(&tessera_hash, &content_key, &published_at);
        let signature = sign_manifest(&message, signing_keys);
        KeyPublication {
            tessera_hash,
            content_key,
            published_at,
            signature,
        }
    }

    /// Verify that a KeyPublication was signed by the expected owner.
    pub fn verify(&self, owner_public: &DualPublicKeys) -> Result<(), CryptoError> {
        let message =
            Self::signing_message(&self.tessera_hash, &self.content_key, &self.published_at);
        verify_manifest(&message, &self.signature, owner_public)
    }

    fn signing_message(
        tessera_hash: &ContentHash,
        content_key: &ContentKey,
        published_at: &chrono::DateTime<chrono::Utc>,
    ) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"tesseras key publication v1\n");
        msg.extend_from_slice(tessera_hash.as_bytes());
        msg.extend_from_slice(content_key);
        msg.extend_from_slice(&published_at.timestamp().to_le_bytes());
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ed25519::Ed25519KeyGenerator;

    #[test]
    fn seal_unseal_roundtrip() {
        let keypair = HybridKem::generate_keypair();
        let public = HybridKem::public_from_keypair(&keypair);
        let content_key: ContentKey = [0x42u8; 32];

        let envelope = SealedKeyEnvelope::seal(&content_key, &public).unwrap();
        let recovered = envelope.unseal(&keypair).unwrap();

        assert_eq!(content_key, recovered);
    }

    #[test]
    fn seal_unseal_wrong_keypair() {
        let keypair1 = HybridKem::generate_keypair();
        let keypair2 = HybridKem::generate_keypair();
        let public1 = HybridKem::public_from_keypair(&keypair1);
        let content_key: ContentKey = [0x42u8; 32];

        let envelope = SealedKeyEnvelope::seal(&content_key, &public1).unwrap();
        let recovered = envelope.unseal(&keypair2).unwrap();

        // Wrong keypair produces wrong content key
        assert_ne!(content_key, recovered);
    }

    #[test]
    fn key_publication_signature_valid() {
        let ed_keypair = Ed25519KeyGenerator::generate();
        let dual_keys = DualKeyPair {
            ed25519: ed_keypair,
            mldsa: None,
        };
        let tessera_hash = ContentHash::new([0xaa; 32]);
        let content_key: ContentKey = [0x42u8; 32];

        let publication = KeyPublication::create(tessera_hash, content_key, &dual_keys);

        let dual_public = DualPublicKeys {
            ed25519: dual_keys.ed25519.verifying_key,
            mldsa: None,
        };
        assert!(publication.verify(&dual_public).is_ok());
    }

    #[test]
    fn unseal_rejects_short_ciphertext() {
        let keypair = HybridKem::generate_keypair();
        // Ciphertext shorter than 32 bytes must fail
        let short_envelope = SealedKeyEnvelope {
            hybrid_ciphertext: HybridCiphertext {
                x25519_ephemeral: [0u8; 32],
                mlkem_ciphertext: vec![0u8; 16], // only 16 bytes, need >= 32
            },
        };
        assert!(matches!(
            short_envelope.unseal(&keypair),
            Err(CryptoError::DecryptFailed)
        ));
    }

    #[test]
    fn unseal_rejects_exactly_31_bytes_ciphertext() {
        let keypair = HybridKem::generate_keypair();
        // Exactly 31 bytes (< 32) must fail
        let envelope = SealedKeyEnvelope {
            hybrid_ciphertext: HybridCiphertext {
                x25519_ephemeral: [0u8; 32],
                mlkem_ciphertext: vec![0u8; 31],
            },
        };
        assert!(matches!(
            envelope.unseal(&keypair),
            Err(CryptoError::DecryptFailed)
        ));
    }

    #[test]
    fn key_publication_tampered_tessera_hash_fails() {
        let ed_keypair = Ed25519KeyGenerator::generate();
        let dual_keys = DualKeyPair {
            ed25519: ed_keypair,
            mldsa: None,
        };
        let tessera_hash = ContentHash::new([0xaa; 32]);
        let content_key: ContentKey = [0x42u8; 32];

        let mut publication = KeyPublication::create(tessera_hash, content_key, &dual_keys);

        let dual_public = DualPublicKeys {
            ed25519: dual_keys.ed25519.verifying_key,
            mldsa: None,
        };
        // Tamper the tessera_hash
        publication.tessera_hash = ContentHash::new([0xbb; 32]);
        assert!(publication.verify(&dual_public).is_err());
    }

    #[test]
    fn key_publication_tampered_content_key_fails() {
        let ed_keypair = Ed25519KeyGenerator::generate();
        let dual_keys = DualKeyPair {
            ed25519: ed_keypair,
            mldsa: None,
        };
        let tessera_hash = ContentHash::new([0xaa; 32]);
        let content_key: ContentKey = [0x42u8; 32];

        let mut publication = KeyPublication::create(tessera_hash, content_key, &dual_keys);

        let dual_public = DualPublicKeys {
            ed25519: dual_keys.ed25519.verifying_key,
            mldsa: None,
        };
        // Tamper the content_key
        publication.content_key = [0x99u8; 32];
        assert!(publication.verify(&dual_public).is_err());
    }

    #[test]
    fn key_publication_tampered_timestamp_fails() {
        let ed_keypair = Ed25519KeyGenerator::generate();
        let dual_keys = DualKeyPair {
            ed25519: ed_keypair,
            mldsa: None,
        };
        let tessera_hash = ContentHash::new([0xaa; 32]);
        let content_key: ContentKey = [0x42u8; 32];

        let mut publication = KeyPublication::create(tessera_hash, content_key, &dual_keys);

        let dual_public = DualPublicKeys {
            ed25519: dual_keys.ed25519.verifying_key,
            mldsa: None,
        };
        // Tamper the timestamp
        publication.published_at = publication.published_at + chrono::Duration::seconds(1);
        assert!(publication.verify(&dual_public).is_err());
    }

    #[test]
    fn key_publication_wrong_signer() {
        let dual_keys1 = DualKeyPair {
            ed25519: Ed25519KeyGenerator::generate(),
            mldsa: None,
        };
        let dual_keys2 = DualKeyPair {
            ed25519: Ed25519KeyGenerator::generate(),
            mldsa: None,
        };
        let tessera_hash = ContentHash::new([0xbb; 32]);
        let content_key: ContentKey = [0x99u8; 32];

        let publication = KeyPublication::create(tessera_hash, content_key, &dual_keys1);

        // Verify with wrong signer's public key
        let wrong_public = DualPublicKeys {
            ed25519: dual_keys2.ed25519.verifying_key,
            mldsa: None,
        };
        assert!(publication.verify(&wrong_public).is_err());
    }
}
