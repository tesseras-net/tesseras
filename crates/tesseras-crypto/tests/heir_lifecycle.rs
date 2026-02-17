//! Integration test: full heir key recovery lifecycle.
//!
//! Tests the complete flow:
//! 1. Generate identity (Ed25519 signing keys)
//! 2. Assemble secret blob with versioned header
//! 3. Split into heir shares
//! 4. Reconstruct from T shares
//! 5. Parse blob back to key material
//! 6. Verify reconstructed Ed25519 public key matches original
//! 7. Sign with reconstructed key, verify with original public key

#![cfg(all(feature = "shamir", feature = "classical"))]

use tesseras_crypto::ed25519::Ed25519KeyGenerator;
use tesseras_crypto::secret_blob;
use tesseras_crypto::shamir::{
    ShamirConfig, ShamirSplitter, share_from_msgpack, share_from_text, share_to_msgpack,
    share_to_text,
};

#[test]
fn heir_full_lifecycle() {
    // 1. Generate identity
    let ed_keypair = Ed25519KeyGenerator::generate();
    let original_public = ed_keypair.verifying_key;
    let original_secret = ed_keypair.signing_key.to_bytes();

    // 2. Assemble secret blob (Ed25519 only)
    let blob = secret_blob::assemble(&original_secret, None, None);

    // 3. Split into 3 shares, threshold 2
    let config = ShamirConfig {
        threshold: 2,
        total_shares: 3,
    };
    let shares = ShamirSplitter::split(&blob, &config, original_public.as_bytes()).unwrap();
    assert_eq!(shares.len(), 3);

    // 4. Reconstruct from shares 0 and 2 (skipping 1)
    let recovered_blob = ShamirSplitter::reconstruct(
        &[shares[0].clone(), shares[2].clone()],
        Some(original_public.as_bytes()),
    )
    .unwrap();

    // 5. Parse blob back to key material
    assert_eq!(recovered_blob, blob);
    let parsed = secret_blob::parse(&recovered_blob).unwrap();

    // 6. Verify public key matches
    let recovered_signing_key = ed25519_dalek::SigningKey::from_bytes(&parsed.ed25519_secret);
    let recovered_public = recovered_signing_key.verifying_key();
    assert_eq!(recovered_public, original_public);

    // 7. Sign with reconstructed key, verify with original public key
    use ed25519_dalek::Signer;
    let message = b"heir recovery successful";
    let signature = recovered_signing_key.sign(message);
    use ed25519_dalek::Verifier;
    original_public.verify(message, &signature).unwrap();
}

#[test]
fn heir_serialization_roundtrip_all_formats() {
    let ed_keypair = Ed25519KeyGenerator::generate();
    let blob = secret_blob::assemble(&ed_keypair.signing_key.to_bytes(), None, None);

    let config = ShamirConfig {
        threshold: 2,
        total_shares: 3,
    };
    let shares =
        ShamirSplitter::split(&blob, &config, ed_keypair.verifying_key.as_bytes()).unwrap();

    for share in &shares {
        // MessagePack roundtrip
        let msgpack = share_to_msgpack(share).unwrap();
        let from_msgpack = share_from_msgpack(&msgpack).unwrap();
        assert_eq!(&from_msgpack, share);

        // Base64 text roundtrip
        let text = share_to_text(share, "2026-02-14").unwrap();
        let from_text = share_from_text(&text).unwrap();
        assert_eq!(&from_text, share);

        // Cross-format: msgpack -> text -> msgpack
        let from_text_msgpack = share_to_msgpack(&from_text).unwrap();
        assert_eq!(from_text_msgpack, msgpack);
    }
}

#[test]
fn heir_reconstruct_then_decrypt_sealed_tessera() {
    // This test requires both shamir + encryption features
    #[cfg(feature = "encryption")]
    {
        use tesseras_core::enums::EncryptionContext;
        use tesseras_crypto::encryption::Aes256GcmEncryptor;

        // Generate identity
        let ed_keypair = Ed25519KeyGenerator::generate();
        let original_secret = ed_keypair.signing_key.to_bytes();
        let blob = secret_blob::assemble(&original_secret, None, None);

        // Create a "content key" and encrypt something
        let content_key: [u8; 32] = [0x42u8; 32];
        let original_content = b"This is a sealed memory that heirs will decrypt.";
        let content_hash =
            tesseras_core::ContentHash::new(*blake3::hash(original_content).as_bytes());
        let ctx = EncryptionContext::Private { content_hash };
        let encrypted = Aes256GcmEncryptor::encrypt(original_content, &content_key, &ctx).unwrap();

        // Split identity keys
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares =
            ShamirSplitter::split(&blob, &config, ed_keypair.verifying_key.as_bytes()).unwrap();

        // Heir reconstructs
        let recovered_blob =
            ShamirSplitter::reconstruct(&[shares[0].clone(), shares[1].clone()], None).unwrap();
        assert_eq!(recovered_blob, blob);

        // Heir uses content key to decrypt (in real use, they'd unseal the envelope first)
        let decrypted = Aes256GcmEncryptor::decrypt(&encrypted, &content_key, &ctx).unwrap();
        assert_eq!(decrypted, original_content);
    }
}

#[test]
#[cfg(feature = "encryption")]
fn heir_full_lifecycle_with_encryption_keys() {
    use tesseras_crypto::kem::HybridKem;

    // 1. Generate all keys
    let ed_keypair = Ed25519KeyGenerator::generate();
    let hybrid = HybridKem::generate_keypair();

    // 2. Assemble full blob
    let ed_secret = ed_keypair.signing_key.to_bytes();
    let x_secret = hybrid.x25519_secret.to_bytes();
    let blob = secret_blob::assemble(&ed_secret, Some(&x_secret), Some(&hybrid.mlkem_secret));

    // 3. Split into 3 shares, threshold 2
    let config = ShamirConfig {
        threshold: 2,
        total_shares: 3,
    };
    let shares =
        ShamirSplitter::split(&blob, &config, ed_keypair.verifying_key.as_bytes()).unwrap();

    // 4. Reconstruct from shares 0 and 2
    let recovered_blob = ShamirSplitter::reconstruct(
        &[shares[0].clone(), shares[2].clone()],
        Some(ed_keypair.verifying_key.as_bytes()),
    )
    .unwrap();
    assert_eq!(recovered_blob, blob);

    // 5. Parse and verify all keys
    let parsed = secret_blob::parse(&recovered_blob).unwrap();
    assert_eq!(parsed.ed25519_secret, ed_secret);
    assert_eq!(parsed.x25519_secret.unwrap(), x_secret);
    assert_eq!(parsed.mlkem768_secret.unwrap(), hybrid.mlkem_secret);

    // 6. Verify Ed25519 public key matches
    let recovered_signing = ed25519_dalek::SigningKey::from_bytes(&parsed.ed25519_secret);
    assert_eq!(recovered_signing.verifying_key(), ed_keypair.verifying_key);
}
