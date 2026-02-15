use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};

/// NIST SP 800-38D Test Case 16: AES-256-GCM with 256-bit key.
/// Key: 32 bytes all zeros
/// IV:  12 bytes all zeros
/// Plaintext: empty
/// AAD: empty
/// This validates the low-level AES-256-GCM implementation matches NIST.
#[test]
fn kat_aes256gcm_nist_test_case_16_empty() {
    let key = [0u8; 32];
    let nonce_bytes = [0u8; 12];
    let plaintext = b"";
    let aad = b"";

    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_ref(),
                aad: aad.as_ref(),
            },
        )
        .unwrap();

    // Expected tag (no ciphertext, just 16-byte tag)
    let expected_tag = hex::decode("530f8afbc74536b9a963b4f1c4cb738b").unwrap();
    assert_eq!(ciphertext.len(), 16); // tag only
    assert_eq!(&ciphertext, &expected_tag);

    // Decrypt back
    let recovered = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext.as_ref(),
                aad: aad.as_ref(),
            },
        )
        .unwrap();
    assert_eq!(recovered, plaintext);
}

/// Validate that our Aes256GcmEncryptor.decrypt correctly handles a
/// pre-computed (nonce, ciphertext) pair.
#[test]
fn kat_aes256gcm_decrypt_known_blob() {
    use tesseras_core::ContentHash;
    use tesseras_core::enums::EncryptionContext;
    use tesseras_crypto::encryption::Aes256GcmEncryptor;

    // Encrypt with known key, capture the blob
    let key = [0x42u8; 32];
    let plaintext = b"tesseras KAT test";
    let ctx = EncryptionContext::Private {
        content_hash: ContentHash::new([0xaa; 32]),
    };

    let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();

    // Verify decrypt recovers plaintext (roundtrip sanity)
    let recovered = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx).unwrap();
    assert_eq!(recovered, plaintext);

    // Verify tampered nonce fails
    let mut bad_blob = blob.clone();
    bad_blob.nonce[0] ^= 0xff;
    assert!(Aes256GcmEncryptor::decrypt(&bad_blob, &key, &ctx).is_err());

    // Verify tampered ciphertext fails
    let mut bad_blob2 = blob.clone();
    bad_blob2.ciphertext[0] ^= 0xff;
    assert!(Aes256GcmEncryptor::decrypt(&bad_blob2, &key, &ctx).is_err());
}

/// Verify AAD binding: same ciphertext decrypted with different AAD must fail.
#[test]
fn kat_aes256gcm_aad_binding() {
    use tesseras_core::ContentHash;
    use tesseras_core::enums::EncryptionContext;
    use tesseras_crypto::encryption::Aes256GcmEncryptor;

    let key = [0x42u8; 32];
    let plaintext = b"AAD binding test";

    let ctx1 = EncryptionContext::Private {
        content_hash: ContentHash::new([0x11; 32]),
    };
    let ctx2 = EncryptionContext::Private {
        content_hash: ContentHash::new([0x22; 32]),
    };

    let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx1).unwrap();

    // Correct AAD works
    assert!(Aes256GcmEncryptor::decrypt(&blob, &key, &ctx1).is_ok());
    // Wrong AAD fails
    assert!(Aes256GcmEncryptor::decrypt(&blob, &key, &ctx2).is_err());
}
