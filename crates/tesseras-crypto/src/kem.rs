use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::{Ciphertext as _, PublicKey as _, SecretKey as _, SharedSecret as _};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519Public, StaticSecret};

use crate::CryptoError;

/// Combined X25519 + ML-KEM-768 keypair for hybrid encryption.
pub struct HybridKeyPair {
    pub x25519_secret: StaticSecret,
    pub x25519_public: X25519Public,
    pub mlkem_secret: Vec<u8>,
    pub mlkem_public: Vec<u8>,
}

/// Public half of a hybrid encryption keypair.
/// Stored in `TesseraIdentity.encryption_public`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridEncryptionPublic {
    pub x25519: [u8; 32],
    pub mlkem768: Vec<u8>,
}

/// Output of encapsulation: ephemeral X25519 public + ML-KEM ciphertext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCiphertext {
    pub x25519_ephemeral: [u8; 32],
    pub mlkem_ciphertext: Vec<u8>,
}

/// 256-bit content encryption key.
pub type ContentKey = [u8; 32];

/// Hybrid Key Encapsulation Mechanism: X25519 + ML-KEM-768.
///
/// Both shared secrets are combined via `blake3::derive_key` to produce
/// a single 256-bit key. Both algorithms must be broken to recover it.
pub struct HybridKem;

impl HybridKem {
    /// Generate a new hybrid keypair.
    pub fn generate_keypair() -> HybridKeyPair {
        let x25519_secret = StaticSecret::random_from_rng(OsRng);
        let x25519_public = X25519Public::from(&x25519_secret);
        let (mlkem_pk, mlkem_sk) = kyber768::keypair();
        HybridKeyPair {
            x25519_secret,
            x25519_public,
            mlkem_secret: mlkem_sk.as_bytes().to_vec(),
            mlkem_public: mlkem_pk.as_bytes().to_vec(),
        }
    }

    /// Extract the public portion of a keypair.
    pub fn public_from_keypair(keypair: &HybridKeyPair) -> HybridEncryptionPublic {
        HybridEncryptionPublic {
            x25519: keypair.x25519_public.to_bytes(),
            mlkem768: keypair.mlkem_public.clone(),
        }
    }

    /// Encapsulate: generate shared secret encrypted to recipient's public key.
    pub fn encapsulate(
        recipient: &HybridEncryptionPublic,
    ) -> Result<(ContentKey, HybridCiphertext), CryptoError> {
        // X25519: ephemeral DH
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_public = X25519Public::from(&ephemeral_secret);
        let recipient_x25519 = X25519Public::from(recipient.x25519);
        let x25519_shared = ephemeral_secret.diffie_hellman(&recipient_x25519);

        // ML-KEM-768: encapsulate
        let mlkem_pk = kyber768::PublicKey::from_bytes(&recipient.mlkem768)
            .map_err(|_| CryptoError::KemFailed("invalid ML-KEM public key".into()))?;
        let (mlkem_shared, mlkem_ct) = kyber768::encapsulate(&mlkem_pk);

        // Combine via BLAKE3 KDF
        let mut ikm = Vec::with_capacity(32 + mlkem_shared.as_bytes().len());
        ikm.extend_from_slice(x25519_shared.as_bytes());
        ikm.extend_from_slice(mlkem_shared.as_bytes());
        let content_key = blake3::derive_key("tesseras hybrid kem v1", &ikm);

        let ciphertext = HybridCiphertext {
            x25519_ephemeral: ephemeral_public.to_bytes(),
            mlkem_ciphertext: mlkem_ct.as_bytes().to_vec(),
        };

        Ok((content_key, ciphertext))
    }

    /// Decapsulate: recover shared secret using recipient's private key.
    pub fn decapsulate(
        ciphertext: &HybridCiphertext,
        keypair: &HybridKeyPair,
    ) -> Result<ContentKey, CryptoError> {
        // X25519: DH with ephemeral public
        let ephemeral_public = X25519Public::from(ciphertext.x25519_ephemeral);
        let x25519_shared = keypair.x25519_secret.diffie_hellman(&ephemeral_public);

        // ML-KEM-768: decapsulate
        let mlkem_sk = kyber768::SecretKey::from_bytes(&keypair.mlkem_secret)
            .map_err(|_| CryptoError::KemFailed("invalid ML-KEM secret key".into()))?;
        let mlkem_ct = kyber768::Ciphertext::from_bytes(&ciphertext.mlkem_ciphertext)
            .map_err(|_| CryptoError::KemFailed("invalid ML-KEM ciphertext".into()))?;
        let mlkem_shared = kyber768::decapsulate(&mlkem_ct, &mlkem_sk);

        // Combine via BLAKE3 KDF (same context string)
        let mut ikm = Vec::with_capacity(32 + mlkem_shared.as_bytes().len());
        ikm.extend_from_slice(x25519_shared.as_bytes());
        ikm.extend_from_slice(mlkem_shared.as_bytes());
        let content_key = blake3::derive_key("tesseras hybrid kem v1", &ikm);

        Ok(content_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_kem_roundtrip() {
        let keypair = HybridKem::generate_keypair();
        let public = HybridKem::public_from_keypair(&keypair);

        let (content_key, ciphertext) = HybridKem::encapsulate(&public).unwrap();
        let recovered_key = HybridKem::decapsulate(&ciphertext, &keypair).unwrap();

        assert_eq!(content_key, recovered_key);
    }

    #[test]
    fn hybrid_kem_wrong_keypair() {
        let keypair1 = HybridKem::generate_keypair();
        let keypair2 = HybridKem::generate_keypair();
        let public1 = HybridKem::public_from_keypair(&keypair1);

        let (content_key, ciphertext) = HybridKem::encapsulate(&public1).unwrap();
        let recovered_key = HybridKem::decapsulate(&ciphertext, &keypair2).unwrap();

        // Different keypair produces different shared secret
        assert_ne!(content_key, recovered_key);
    }

    #[test]
    fn hybrid_kem_tampered_x25519_ciphertext() {
        let keypair = HybridKem::generate_keypair();
        let public = HybridKem::public_from_keypair(&keypair);

        let (content_key, mut ciphertext) = HybridKem::encapsulate(&public).unwrap();
        ciphertext.x25519_ephemeral[0] ^= 0xff;

        let recovered_key = HybridKem::decapsulate(&ciphertext, &keypair).unwrap();
        // Tampered X25519 ephemeral produces wrong shared secret
        assert_ne!(content_key, recovered_key);
    }

    #[test]
    fn derive_key_deterministic() {
        let ikm = [0x01u8; 64];
        let key1 = blake3::derive_key("tesseras hybrid kem v1", &ikm);
        let key2 = blake3::derive_key("tesseras hybrid kem v1", &ikm);
        assert_eq!(key1, key2);

        // Different context string produces different key
        let key3 = blake3::derive_key("tesseras hybrid kem v2", &ikm);
        assert_ne!(key1, key3);
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(10))] // KEM keygen is slow
            #[test]
            fn prop_hybrid_kem_any_keypair(_seed in 0u64..1000) {
                let keypair = HybridKem::generate_keypair();
                let public = HybridKem::public_from_keypair(&keypair);
                let (content_key, ciphertext) = HybridKem::encapsulate(&public).unwrap();
                let recovered = HybridKem::decapsulate(&ciphertext, &keypair).unwrap();
                prop_assert_eq!(content_key, recovered);
            }
        }
    }
}
