//! Integration test: full sealed tessera encryption lifecycle.
//!
//! Tests the complete flow:
//! 1. Generate identity with encryption keys
//! 2. Create random content key
//! 3. Encrypt memory content with AES-256-GCM + EncryptionContext::Sealed AAD
//! 4. Seal content key in envelope with HybridKem
//! 5. Unseal envelope to recover content key
//! 6. Decrypt memory content
//! 7. Verify recovered content matches original
//! 8. Create KeyPublication and verify signature

#![cfg(feature = "encryption")]

use chrono::TimeZone;
use tesseras_core::ContentHash;
use tesseras_core::enums::EncryptionContext;
use tesseras_crypto::dual::{DualKeyPair, DualPublicKeys};
use tesseras_crypto::ed25519::Ed25519KeyGenerator;
use tesseras_crypto::encryption::Aes256GcmEncryptor;
use tesseras_crypto::kem::{ContentKey, HybridKem};
use tesseras_crypto::sealed::{KeyPublication, SealedKeyEnvelope};

#[test]
fn sealed_tessera_full_cycle() {
    // 1. Generate identity: signing keys + encryption keys
    let signing_keys = DualKeyPair {
        ed25519: Ed25519KeyGenerator::generate(),
        mldsa: None,
    };
    let encryption_keypair = HybridKem::generate_keypair();
    let encryption_public = HybridKem::public_from_keypair(&encryption_keypair);

    // 2. Create random content key
    let content_key: ContentKey = rand::random();

    // 3. Encrypt memory content
    let original_content = b"This is a sealed memory from 2026, to be opened in 2050.";
    let content_hash = ContentHash::new(*blake3::hash(original_content).as_bytes());
    let open_after = chrono::Utc.with_ymd_and_hms(2050, 1, 1, 0, 0, 0).unwrap();
    let ctx = EncryptionContext::Sealed {
        content_hash,
        open_after,
    };
    let encrypted = Aes256GcmEncryptor::encrypt(original_content, &content_key, &ctx).unwrap();

    // 4. Seal content key in envelope
    let envelope = SealedKeyEnvelope::seal(&content_key, &encryption_public).unwrap();

    // --- Simulate time passing / storage / retrieval ---

    // 5. Unseal envelope to recover content key
    let recovered_key = envelope.unseal(&encryption_keypair).unwrap();
    assert_eq!(content_key, recovered_key);

    // 6. Decrypt memory content
    let decrypted = Aes256GcmEncryptor::decrypt(&encrypted, &recovered_key, &ctx).unwrap();
    assert_eq!(decrypted, original_content);

    // 7. Create KeyPublication (owner publishes key after open_after)
    let publication = KeyPublication::create(content_hash, recovered_key, &signing_keys);

    // 8. "Another node" verifies the KeyPublication
    let owner_public = DualPublicKeys {
        ed25519: signing_keys.ed25519.verifying_key,
        mldsa: None,
    };
    publication.verify(&owner_public).unwrap();

    // 9. "Another node" decrypts using the published key
    let other_node_decrypted =
        Aes256GcmEncryptor::decrypt(&encrypted, &publication.content_key, &ctx).unwrap();
    assert_eq!(other_node_decrypted, original_content);
}

#[test]
fn private_tessera_full_cycle() {
    // Generate keys
    let encryption_keypair = HybridKem::generate_keypair();
    let encryption_public = HybridKem::public_from_keypair(&encryption_keypair);

    // Create and encrypt
    let content_key: ContentKey = rand::random();
    let original_content = b"This is a private memory, never to be published.";
    let content_hash = ContentHash::new(*blake3::hash(original_content).as_bytes());
    let ctx = EncryptionContext::Private { content_hash };
    let encrypted = Aes256GcmEncryptor::encrypt(original_content, &content_key, &ctx).unwrap();

    // Seal
    let envelope = SealedKeyEnvelope::seal(&content_key, &encryption_public).unwrap();

    // Only owner can unseal and decrypt
    let recovered_key = envelope.unseal(&encryption_keypair).unwrap();
    let decrypted = Aes256GcmEncryptor::decrypt(&encrypted, &recovered_key, &ctx).unwrap();
    assert_eq!(decrypted, original_content);

    // No KeyPublication for Private — content key is never published
}
