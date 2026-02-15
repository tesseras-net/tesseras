//! E2E: generate keys → create sealed tessera → unseal envelope → decrypt content

#![cfg(all(feature = "encryption", feature = "shamir"))]

use tesseras_core::enums::EncryptionContext;
use tesseras_core::ContentHash;
use tesseras_crypto::encryption::Aes256GcmEncryptor;
use tesseras_crypto::kem::{HybridKem, HybridKeyPair};
use tesseras_crypto::sealed::SealedKeyEnvelope;
use tesseras_crypto::secret_blob;
use tesseras_crypto::shamir::{ShamirConfig, ShamirSplitter};

#[test]
fn sealed_tessera_create_and_heir_decrypt() {
    // 1. Owner generates all keys
    let ed_keypair = tesseras_crypto::ed25519::Ed25519KeyGenerator::generate();
    let hybrid = HybridKem::generate_keypair();
    let hybrid_public = HybridKem::public_from_keypair(&hybrid);

    // 2. Owner creates sealed tessera — encrypt content
    let content_key: [u8; 32] = rand::random();
    let original = b"This memory opens in 2050";
    let content_hash = ContentHash::new(*blake3::hash(original).as_bytes());
    let ctx = EncryptionContext::Sealed {
        content_hash,
        open_after: chrono::Utc::now() + chrono::Duration::days(365 * 24),
    };
    let encrypted = Aes256GcmEncryptor::encrypt(original, &content_key, &ctx).unwrap();

    // 3. Seal content key to owner's public key
    let envelope = SealedKeyEnvelope::seal(&content_key, &hybrid_public).unwrap();

    // 4. Owner creates heir shares with full key material
    let ed_secret = ed_keypair.signing_key.to_bytes();
    let x_secret = hybrid.x25519_secret.to_bytes();
    let blob = secret_blob::assemble(&ed_secret, Some(&x_secret), Some(&hybrid.mlkem_secret));
    let config = ShamirConfig {
        threshold: 2,
        total_shares: 3,
    };
    let shares =
        ShamirSplitter::split(&blob, &config, ed_keypair.verifying_key.as_bytes()).unwrap();

    // 5. Owner dies. Heirs reconstruct with 2 of 3 shares.
    let recovered_blob =
        ShamirSplitter::reconstruct(&[shares[0].clone(), shares[2].clone()], None).unwrap();
    let parsed = secret_blob::parse(&recovered_blob).unwrap();

    // 6. Heirs rebuild HybridKeyPair from recovered secrets
    let recovered_x_secret =
        x25519_dalek::StaticSecret::from(parsed.x25519_secret.expect("x25519 present"));
    let recovered_x_public = x25519_dalek::PublicKey::from(&recovered_x_secret);
    let recovered_hybrid = HybridKeyPair {
        x25519_secret: recovered_x_secret,
        x25519_public: recovered_x_public,
        mlkem_secret: parsed.mlkem768_secret.expect("mlkem768 present"),
        mlkem_public: hybrid.mlkem_public.clone(), // heirs get this from identity store
    };

    // 7. Heirs unseal the envelope
    let recovered_content_key = envelope.unseal(&recovered_hybrid).unwrap();
    assert_eq!(content_key, recovered_content_key);

    // 8. Heirs decrypt the content
    let decrypted =
        Aes256GcmEncryptor::decrypt(&encrypted, &recovered_content_key, &ctx).unwrap();
    assert_eq!(decrypted, original);
}
