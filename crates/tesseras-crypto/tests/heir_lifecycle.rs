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
use tesseras_crypto::shamir::{
    ShamirConfig, ShamirSplitter, share_from_msgpack, share_from_text, share_to_msgpack,
    share_to_text,
};

/// Secret blob layout:
/// byte 0: version (0x01)
/// byte 1: flags (bit 0 = has_x25519, bit 1 = has_mlkem768)
/// bytes 2-33: ed25519_secret (32 bytes, always present)
fn assemble_secret_blob(ed25519_secret: &[u8; 32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(34);
    blob.push(0x01); // version
    blob.push(0x00); // flags: no x25519, no mlkem768
    blob.extend_from_slice(ed25519_secret);
    blob
}

fn parse_ed25519_secret(blob: &[u8]) -> &[u8; 32] {
    assert!(blob.len() >= 34);
    assert_eq!(blob[0], 0x01, "unexpected version");
    blob[2..34].try_into().unwrap()
}

#[test]
fn heir_full_lifecycle() {
    // 1. Generate identity
    let ed_keypair = Ed25519KeyGenerator::generate();
    let original_public = ed_keypair.verifying_key;
    let original_secret = ed_keypair.signing_key.to_bytes();

    // 2. Assemble secret blob
    let blob = assemble_secret_blob(&original_secret);

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
    let recovered_secret = parse_ed25519_secret(&recovered_blob);

    // 6. Verify public key matches
    let recovered_signing_key = ed25519_dalek::SigningKey::from_bytes(recovered_secret);
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
    let blob = assemble_secret_blob(&ed_keypair.signing_key.to_bytes());

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
        let blob = assemble_secret_blob(&original_secret);

        // Create a "content key" and encrypt something
        let content_key: [u8; 32] = [0x42u8; 32];
        let original_content = b"This is a sealed memory that heirs will decrypt.";
        let content_hash =
            tesseras_core::ContentHash::new(*blake3::hash(original_content).as_bytes());
        let ctx = EncryptionContext::Sealed {
            content_hash,
            open_after: chrono::Utc::now(),
        };
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
