use ed25519_dalek::SigningKey;
use tesseras_crypto::ed25519::{Ed25519Signer, Ed25519Verifier};

/// RFC 8032 Section 7.1 — Test Vector 1 (empty message).
#[test]
fn kat_ed25519_rfc8032_test1() {
    let secret_bytes =
        hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").unwrap();
    let expected_public =
        hex::decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a").unwrap();
    let message = b""; // empty
    let expected_sig_bytes = hex::decode(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
         5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    )
    .unwrap();

    let signing_key = SigningKey::from_bytes(secret_bytes.as_slice().try_into().unwrap());
    let verifying_key = signing_key.verifying_key();

    // Verify public key derivation matches RFC 8032
    assert_eq!(verifying_key.as_bytes(), expected_public.as_slice());

    // Sign and verify against known signature
    let sig = Ed25519Signer::sign(message, &signing_key);
    assert_eq!(sig.to_bytes().as_slice(), expected_sig_bytes.as_slice());

    // Verify passes
    assert!(Ed25519Verifier::verify(message, &sig, &verifying_key).is_ok());
}

/// RFC 8032 Section 7.1 — Test Vector 2 (1-byte message 0x72).
#[test]
fn kat_ed25519_rfc8032_test2() {
    let secret_bytes =
        hex::decode("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb").unwrap();
    let expected_public =
        hex::decode("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c").unwrap();
    let message = hex::decode("72").unwrap();
    let expected_sig_bytes = hex::decode(
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da\
         085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    )
    .unwrap();

    let signing_key = SigningKey::from_bytes(secret_bytes.as_slice().try_into().unwrap());
    let verifying_key = signing_key.verifying_key();
    assert_eq!(verifying_key.as_bytes(), expected_public.as_slice());

    let sig = Ed25519Signer::sign(&message, &signing_key);
    assert_eq!(sig.to_bytes().as_slice(), expected_sig_bytes.as_slice());
    assert!(Ed25519Verifier::verify(&message, &sig, &verifying_key).is_ok());
}

/// RFC 8032 Section 7.1 — Test Vector 3 (2-byte message).
#[test]
fn kat_ed25519_rfc8032_test3() {
    let secret_bytes =
        hex::decode("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7").unwrap();
    let expected_public =
        hex::decode("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025").unwrap();
    let message = hex::decode("af82").unwrap();
    let expected_sig_bytes = hex::decode(
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac\
         18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    )
    .unwrap();

    let signing_key = SigningKey::from_bytes(secret_bytes.as_slice().try_into().unwrap());
    let verifying_key = signing_key.verifying_key();
    assert_eq!(verifying_key.as_bytes(), expected_public.as_slice());

    let sig = Ed25519Signer::sign(&message, &signing_key);
    assert_eq!(sig.to_bytes().as_slice(), expected_sig_bytes.as_slice());
    assert!(Ed25519Verifier::verify(&message, &sig, &verifying_key).is_ok());
}
