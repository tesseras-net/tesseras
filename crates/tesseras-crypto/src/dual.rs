use ed25519_dalek::{Signature as Ed25519Sig, VerifyingKey};

use crate::CryptoError;
use crate::ed25519::{Ed25519KeyPair, Ed25519Signer, Ed25519Verifier};

pub struct DualKeyPair {
    pub ed25519: Ed25519KeyPair,
    pub mldsa: Option<()>, // placeholder, real type behind post-quantum flag
}

pub struct DualPublicKeys {
    pub ed25519: VerifyingKey,
    pub mldsa: Option<()>, // placeholder
}

#[derive(Debug, Clone)]
pub struct DualSignature {
    pub ed25519: Ed25519Sig,
    pub mldsa: Option<Vec<u8>>, // raw bytes placeholder
}

pub fn sign_manifest(manifest: &[u8], keys: &DualKeyPair) -> DualSignature {
    let ed_sig = Ed25519Signer::sign(manifest, &keys.ed25519.signing_key);
    DualSignature {
        ed25519: ed_sig,
        mldsa: None, // ML-DSA signing when post-quantum enabled
    }
}

pub fn verify_manifest(
    manifest: &[u8],
    sig: &DualSignature,
    keys: &DualPublicKeys,
) -> Result<(), CryptoError> {
    // Ed25519 is always required
    Ed25519Verifier::verify(manifest, &sig.ed25519, &keys.ed25519)?;

    // Reject: ML-DSA signature present but no ML-DSA key
    if sig.mldsa.is_some() && keys.mldsa.is_none() {
        return Err(CryptoError::InvalidKey(
            "ML-DSA signature present but no ML-DSA public key provided".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ed25519::Ed25519KeyGenerator;

    #[test]
    fn sign_verify_manifest_ed25519_only() {
        let ed_pair = Ed25519KeyGenerator::generate();
        let verifying_key = ed_pair.verifying_key;
        let dual_keys = DualKeyPair {
            ed25519: ed_pair,
            mldsa: None,
        };
        let manifest = b"TESSERA MANIFEST v1\ncontent here";
        let sig = sign_manifest(manifest, &dual_keys);
        let pub_keys = DualPublicKeys {
            ed25519: verifying_key,
            mldsa: None,
        };
        assert!(verify_manifest(manifest, &sig, &pub_keys).is_ok());
    }

    #[test]
    fn verify_manifest_rejects_tampered() {
        let ed_pair = Ed25519KeyGenerator::generate();
        let verifying_key = ed_pair.verifying_key;
        let dual_keys = DualKeyPair {
            ed25519: ed_pair,
            mldsa: None,
        };
        let sig = sign_manifest(b"original", &dual_keys);
        let pub_keys = DualPublicKeys {
            ed25519: verifying_key,
            mldsa: None,
        };
        assert!(verify_manifest(b"tampered", &sig, &pub_keys).is_err());
    }

    #[test]
    fn verify_rejects_mldsa_sig_without_mldsa_key() {
        let ed_pair = Ed25519KeyGenerator::generate();
        let verifying_key = ed_pair.verifying_key;
        let dual_keys = DualKeyPair {
            ed25519: ed_pair,
            mldsa: None,
        };
        let mut sig = sign_manifest(b"data", &dual_keys);
        sig.mldsa = Some(vec![0u8; 64]); // fake sig bytes
        let pub_keys = DualPublicKeys {
            ed25519: verifying_key,
            mldsa: None,
        };
        assert!(verify_manifest(b"data", &sig, &pub_keys).is_err());
    }
}
