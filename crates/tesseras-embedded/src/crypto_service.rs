//! Adapter implementations of tesseras-core service traits
//! wrapping tesseras-crypto primitives.

use ed25519_dalek::{Signer, Verifier};
use tesseras_core::ContentHash;
use tesseras_core::ports::{Hasher, ManifestSigner, ManifestVerifier};

/// Wraps Blake3Hasher to implement the Hasher port trait.
pub struct Blake3HasherAdapter;

impl Hasher for Blake3HasherAdapter {
    fn hash(&self, data: &[u8]) -> ContentHash {
        tesseras_crypto::hasher::Blake3Hasher::hash(data)
    }
}

/// Wraps an Ed25519 signing key to implement ManifestSigner.
pub struct Ed25519SignerAdapter {
    signing_key: ed25519_dalek::SigningKey,
}

impl Ed25519SignerAdapter {
    pub fn new(signing_key: ed25519_dalek::SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn pub_key_hex(&self) -> String {
        self.signing_key
            .verifying_key()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

impl ManifestSigner for Ed25519SignerAdapter {
    fn sign(&self, manifest: &[u8]) -> (Vec<u8>, String) {
        let sig = self.signing_key.sign(manifest);
        (sig.to_bytes().to_vec(), self.pub_key_hex())
    }
}

/// Implements ManifestVerifier using Ed25519.
pub struct Ed25519VerifierAdapter;

impl ManifestVerifier for Ed25519VerifierAdapter {
    fn verify(&self, manifest: &[u8], signature: &[u8], public_key_hex: &str) -> bool {
        if signature.len() != 64 {
            return false;
        }
        let sig_array: [u8; 64] = match signature.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let sig = ed25519_dalek::Signature::from_bytes(&sig_array);
        let pub_bytes: Vec<u8> = (0..public_key_hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok())
            .collect();
        if pub_bytes.len() != 32 {
            return false;
        }
        let pub_array: [u8; 32] = match pub_bytes.try_into() {
            Ok(a) => a,
            Err(_) => return false,
        };
        if let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pub_array) {
            vk.verify(manifest, &sig).is_ok()
        } else {
            false
        }
    }
}
