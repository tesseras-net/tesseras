use tesseras_core::ContentHash;
use tesseras_core::enums::EncryptionContext;
use tesseras_crypto::encryption::Aes256GcmEncryptor;

#[test]
fn fuzz_sealed_nonce_uniqueness() {
    bolero::check!()
        .with_type::<Vec<u8>>()
        .for_each(|plaintext| {
            let key = [0x42u8; 32];
            let ctx = EncryptionContext::Private {
                content_hash: ContentHash::new([0xaa; 32]),
            };

            // Encrypt same plaintext twice — nonces must differ
            let blob1 = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
            let blob2 = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();

            assert_ne!(blob1.nonce, blob2.nonce, "Nonce reuse detected!");
        });
}

#[test]
fn fuzz_sealed_encrypt_decrypt_roundtrip() {
    bolero::check!()
        .with_type::<Vec<u8>>()
        .for_each(|plaintext| {
            let key = [0x42u8; 32];
            let ctx = EncryptionContext::Private {
                content_hash: ContentHash::new([0xaa; 32]),
            };

            let blob = Aes256GcmEncryptor::encrypt(plaintext, &key, &ctx).unwrap();
            let recovered = Aes256GcmEncryptor::decrypt(&blob, &key, &ctx).unwrap();
            assert_eq!(&recovered, plaintext);
        });
}
