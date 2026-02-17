use std::path::Path;

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use zeroize::Zeroize;

use crate::types::{ContentHash, NodeId};

/// Hash a byte slice with BLAKE3.
pub fn hash_bytes(data: &[u8]) -> ContentHash {
    let hash = blake3::hash(data);
    ContentHash::new(*hash.as_bytes())
}

/// Hash a file by streaming (no full load into memory).
pub fn hash_file(path: &Path) -> std::io::Result<ContentHash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(std::fs::File::open(path)?)?;
    Ok(ContentHash::new(*hasher.finalize().as_bytes()))
}

/// Identity keypair (Ed25519).
pub struct Identity {
    signing_key: SigningKey,
}

impl Identity {
    /// Generate a new random identity.
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            signing_key: SigningKey::generate(&mut rng),
        }
    }

    /// Load identity from a key file (32-byte secret key).
    pub fn load(path: &Path) -> Result<Self, CryptoError> {
        let mut bytes = std::fs::read(path).map_err(|e| CryptoError::Io(e.to_string()))?;
        if bytes.len() != 32 {
            bytes.zeroize();
            return Err(CryptoError::InvalidKeyLength);
        }
        let arr: [u8; 32] = bytes[..32].try_into().unwrap();
        bytes.zeroize();
        Ok(Self {
            signing_key: SigningKey::from_bytes(&arr),
        })
    }

    /// Save identity to a key file (32-byte secret key).
    pub fn save(&self, path: &Path) -> Result<(), CryptoError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CryptoError::Io(e.to_string()))?;
        }
        std::fs::write(path, self.signing_key.to_bytes())
            .map_err(|e| CryptoError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| CryptoError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// Get the public key bytes.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    /// Derive NodeId from public key: BLAKE3(public_key).
    pub fn node_id(&self) -> NodeId {
        let pk = self.signing_key.verifying_key().to_bytes();
        let hash = blake3::hash(&pk);
        NodeId::new(*hash.as_bytes())
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Create a signed envelope wrapping serialized data.
    pub fn sign_envelope(&self, payload: Vec<u8>) -> crate::dht::SignedEnvelope {
        let signature = self.sign(&payload);
        crate::dht::SignedEnvelope {
            payload,
            public_key: self.public_key_bytes(),
            signature,
        }
    }

    /// Verify a signature against a public key.
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        let pk_bytes: [u8; 32] = public_key
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let verifying_key =
            VerifyingKey::from_bytes(&pk_bytes).map_err(|_| CryptoError::InvalidKey)?;
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(message, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

impl Drop for Identity {
    fn drop(&mut self) {
        // SigningKey implements Zeroize internally via ed25519-dalek
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid key length")]
    InvalidKeyLength,
    #[error("invalid key")]
    InvalidKey,
    #[error("invalid signature")]
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_bytes_deterministic() {
        let h1 = hash_bytes(b"hello tesseras");
        let h2 = hash_bytes(b"hello tesseras");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_bytes_different_inputs() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_file_matches_bytes() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"test data").unwrap();
        let file_hash = hash_file(tmp.path()).unwrap();
        let bytes_hash = hash_bytes(b"test data");
        assert_eq!(file_hash, bytes_hash);
    }

    #[test]
    fn identity_generate_and_sign() {
        let id = Identity::generate();
        let message = b"tessera content";
        let sig = id.sign(message);
        let pk = id.public_key_bytes();
        Identity::verify(&pk, message, &sig).unwrap();
    }

    #[test]
    fn identity_verify_wrong_message() {
        let id = Identity::generate();
        let sig = id.sign(b"correct");
        let pk = id.public_key_bytes();
        assert!(Identity::verify(&pk, b"wrong", &sig).is_err());
    }

    #[test]
    fn identity_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("identity.key");

        let id1 = Identity::generate();
        id1.save(&path).unwrap();

        let id2 = Identity::load(&path).unwrap();
        assert_eq!(id1.public_key_bytes(), id2.public_key_bytes());
    }

    #[test]
    fn node_id_from_public_key() {
        let id = Identity::generate();
        let nid = id.node_id();
        let expected = blake3::hash(&id.public_key_bytes());
        assert_eq!(nid.as_bytes(), expected.as_bytes());
    }
}
