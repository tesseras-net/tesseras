use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

use crate::CryptoError;

pub struct Ed25519KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

pub struct Ed25519KeyGenerator;

impl Ed25519KeyGenerator {
    pub fn generate() -> Ed25519KeyPair {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Ed25519KeyPair {
            signing_key,
            verifying_key,
        }
    }
}

pub struct Ed25519Signer;

impl Ed25519Signer {
    pub fn sign(data: &[u8], key: &SigningKey) -> Signature {
        key.sign(data)
    }
}

pub struct Ed25519Verifier;

impl Ed25519Verifier {
    pub fn verify(data: &[u8], sig: &Signature, key: &VerifyingKey) -> Result<(), CryptoError> {
        key.verify(data, sig)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_sign_verify_roundtrip() {
        let keypair = Ed25519KeyGenerator::generate();
        let data = b"tessera manifest content";
        let sig = Ed25519Signer::sign(data, &keypair.signing_key);
        let result = Ed25519Verifier::verify(data, &sig, &keypair.verifying_key);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_rejects_tampered_data() {
        let keypair = Ed25519KeyGenerator::generate();
        let sig = Ed25519Signer::sign(b"original", &keypair.signing_key);
        let result = Ed25519Verifier::verify(b"tampered", &sig, &keypair.verifying_key);
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let keypair1 = Ed25519KeyGenerator::generate();
        let keypair2 = Ed25519KeyGenerator::generate();
        let sig = Ed25519Signer::sign(b"data", &keypair1.signing_key);
        let result = Ed25519Verifier::verify(b"data", &sig, &keypair2.verifying_key);
        assert!(result.is_err());
    }

    #[test]
    fn keypair_to_bytes_roundtrip() {
        let keypair = Ed25519KeyGenerator::generate();
        let secret_bytes = keypair.signing_key.to_bytes();
        let public_bytes = keypair.verifying_key.to_bytes();
        let restored_signing = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let restored_verifying =
            ed25519_dalek::VerifyingKey::from_bytes(&public_bytes).unwrap();
        let sig = Ed25519Signer::sign(b"test", &restored_signing);
        assert!(Ed25519Verifier::verify(b"test", &sig, &restored_verifying).is_ok());
    }
}
