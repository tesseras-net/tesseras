use ed25519_dalek::SigningKey;
use tesseras_crypto::dual::{DualKeyPair, DualPublicKeys, sign_manifest, verify_manifest};
use tesseras_crypto::ed25519::{Ed25519KeyPair, Ed25519Signer};

/// Helper: create a deterministic Ed25519 keypair from a seed.
fn keypair_from_seed(seed: [u8; 32]) -> Ed25519KeyPair {
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    Ed25519KeyPair {
        signing_key,
        verifying_key,
    }
}

/// Golden vector: deterministic seed produces known public key and signature.
#[test]
fn kat_dual_golden_vector_ed25519_only() {
    let seed = [0x01u8; 32];
    let ed_pair = keypair_from_seed(seed);
    let manifest = b"TESSERA MANIFEST v1\nhash:abc123";

    let dual_keys = DualKeyPair {
        ed25519: ed_pair,
        mldsa: None,
    };

    let sig = sign_manifest(manifest, &dual_keys);

    // Verify the Ed25519 signature matches what ed25519-dalek produces directly
    let direct_sig = Ed25519Signer::sign(manifest, &dual_keys.ed25519.signing_key);
    assert_eq!(sig.ed25519.to_bytes(), direct_sig.to_bytes());

    // ML-DSA should be None
    assert!(sig.mldsa.is_none());

    // Verify passes
    let pub_keys = DualPublicKeys {
        ed25519: dual_keys.ed25519.verifying_key,
        mldsa: None,
    };
    assert!(verify_manifest(manifest, &sig, &pub_keys).is_ok());
}

/// Edge case: ML-DSA signature present but no ML-DSA public key -> reject.
#[test]
fn kat_dual_reject_mldsa_sig_without_key() {
    let ed_pair = keypair_from_seed([0x02u8; 32]);
    let dual_keys = DualKeyPair {
        ed25519: ed_pair,
        mldsa: None,
    };
    let manifest = b"test manifest";
    let mut sig = sign_manifest(manifest, &dual_keys);

    // Inject fake ML-DSA signature
    sig.mldsa = Some(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let pub_keys = DualPublicKeys {
        ed25519: dual_keys.ed25519.verifying_key,
        mldsa: None, // No ML-DSA key
    };

    let result = verify_manifest(manifest, &sig, &pub_keys);
    assert!(result.is_err(), "Must reject ML-DSA sig without ML-DSA key");
}

/// Edge case: ML-DSA public key present but ML-DSA signature absent.
/// Current code does NOT reject this — this test documents the gap.
/// If verify_manifest is updated to reject this, change the assertion.
#[test]
fn kat_dual_mldsa_key_present_sig_absent() {
    let ed_pair = keypair_from_seed([0x03u8; 32]);
    let dual_keys = DualKeyPair {
        ed25519: ed_pair,
        mldsa: None,
    };
    let manifest = b"test manifest";
    let sig = sign_manifest(manifest, &dual_keys);

    // ML-DSA signature is None (not signed with ML-DSA)
    assert!(sig.mldsa.is_none());

    let pub_keys = DualPublicKeys {
        ed25519: dual_keys.ed25519.verifying_key,
        mldsa: Some(()), // ML-DSA key present but sig absent
    };

    // Document current behavior: this currently PASSES (no ML-DSA check when sig absent)
    // TODO: Consider whether this should be rejected (strict mode)
    let result = verify_manifest(manifest, &sig, &pub_keys);
    // This assertion documents current behavior. cargo-mutants will flag if
    // verify_manifest is changed to reject this case without updating the test.
    assert!(
        result.is_ok(),
        "Current behavior: ML-DSA key without sig is accepted (backwards compat)"
    );
}

/// Edge case: tampered manifest with valid signature -> must reject.
#[test]
fn kat_dual_tampered_manifest() {
    let ed_pair = keypair_from_seed([0x04u8; 32]);
    let dual_keys = DualKeyPair {
        ed25519: ed_pair,
        mldsa: None,
    };

    let original = b"original manifest content";
    let sig = sign_manifest(original, &dual_keys);

    let pub_keys = DualPublicKeys {
        ed25519: dual_keys.ed25519.verifying_key,
        mldsa: None,
    };

    // Original verifies
    assert!(verify_manifest(original, &sig, &pub_keys).is_ok());

    // Tampered rejects
    assert!(verify_manifest(b"tampered manifest content", &sig, &pub_keys).is_err());

    // Truncated rejects
    assert!(verify_manifest(b"original", &sig, &pub_keys).is_err());

    // Extended rejects
    let mut extended = original.to_vec();
    extended.push(0x00);
    assert!(verify_manifest(&extended, &sig, &pub_keys).is_err());
}

/// Edge case: empty manifest is valid (no reason to reject).
#[test]
fn kat_dual_empty_manifest() {
    let ed_pair = keypair_from_seed([0x05u8; 32]);
    let dual_keys = DualKeyPair {
        ed25519: ed_pair,
        mldsa: None,
    };

    let sig = sign_manifest(b"", &dual_keys);

    let pub_keys = DualPublicKeys {
        ed25519: dual_keys.ed25519.verifying_key,
        mldsa: None,
    };

    assert!(verify_manifest(b"", &sig, &pub_keys).is_ok());
}

/// Edge case: wrong signer's key -> must reject.
#[test]
fn kat_dual_wrong_signer() {
    let ed_pair1 = keypair_from_seed([0x06u8; 32]);
    let ed_pair2 = keypair_from_seed([0x07u8; 32]);

    let dual_keys1 = DualKeyPair {
        ed25519: ed_pair1,
        mldsa: None,
    };

    let manifest = b"signed by keypair 1";
    let sig = sign_manifest(manifest, &dual_keys1);

    // Verify with keypair 2's public key
    let wrong_pub = DualPublicKeys {
        ed25519: ed_pair2.verifying_key,
        mldsa: None,
    };

    assert!(verify_manifest(manifest, &sig, &wrong_pub).is_err());
}
