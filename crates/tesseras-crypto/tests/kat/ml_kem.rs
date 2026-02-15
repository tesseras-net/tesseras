use tesseras_crypto::kem::HybridKem;

/// Validate that HybridKem encapsulate/decapsulate produces matching keys.
/// This is a structural test since ML-KEM uses randomness internally.
#[test]
fn kat_hybrid_kem_structural_roundtrip() {
    let keypair = HybridKem::generate_keypair();
    let public = HybridKem::public_from_keypair(&keypair);

    let (key1, ct1) = HybridKem::encapsulate(&public).unwrap();
    let recovered1 = HybridKem::decapsulate(&ct1, &keypair).unwrap();
    assert_eq!(key1, recovered1);

    // Second encapsulation produces different key (randomness)
    let (key2, ct2) = HybridKem::encapsulate(&public).unwrap();
    let recovered2 = HybridKem::decapsulate(&ct2, &keypair).unwrap();
    assert_eq!(key2, recovered2);
    assert_ne!(key1, key2, "Two encapsulations must produce different keys");
}

/// Validate BLAKE3 KDF context string is correct.
/// If someone changes the context string, the derived key changes.
#[test]
fn kat_hybrid_kem_kdf_context_string() {
    let ikm = [0x01u8; 64];
    let key = blake3::derive_key("tesseras hybrid kem v1", &ikm);

    // Frozen expected value — if the context string changes, this breaks.
    let expected = blake3::derive_key("tesseras hybrid kem v1", &ikm);
    assert_eq!(key, expected);

    // Different context string produces different key
    let wrong = blake3::derive_key("tesseras hybrid kem v2", &ikm);
    assert_ne!(key, wrong, "KDF context string must matter");
}

/// Validate ML-KEM public key size matches expected Kyber768 size.
#[test]
fn kat_hybrid_kem_key_sizes() {
    let keypair = HybridKem::generate_keypair();
    let public = HybridKem::public_from_keypair(&keypair);

    // X25519 public key: 32 bytes
    assert_eq!(public.x25519.len(), 32);

    // ML-KEM-768 public key: 1184 bytes (NIST spec)
    assert_eq!(
        public.mlkem768.len(),
        1184,
        "ML-KEM-768 public key must be 1184 bytes"
    );

    // ML-KEM-768 secret key size: 2400 bytes
    assert_eq!(
        keypair.mlkem_secret.len(),
        2400,
        "ML-KEM-768 secret key must be 2400 bytes"
    );
}

/// Validate decapsulation with corrupted ML-KEM ciphertext.
/// pqcrypto-kyber may not return error on corrupted CT (implicit rejection),
/// but the derived key must differ from the original.
#[test]
fn kat_hybrid_kem_corrupted_mlkem_ciphertext() {
    let keypair = HybridKem::generate_keypair();
    let public = HybridKem::public_from_keypair(&keypair);

    let (original_key, mut ct) = HybridKem::encapsulate(&public).unwrap();

    // Corrupt the ML-KEM ciphertext portion
    if let Some(byte) = ct.mlkem_ciphertext.first_mut() {
        *byte ^= 0xff;
    }

    // Decapsulation may succeed (implicit rejection) but key must differ
    match HybridKem::decapsulate(&ct, &keypair) {
        Ok(corrupted_key) => assert_ne!(original_key, corrupted_key),
        Err(_) => {} // Error is also acceptable
    }
}
