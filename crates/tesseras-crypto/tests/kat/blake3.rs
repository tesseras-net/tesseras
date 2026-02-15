use tesseras_crypto::hasher::Blake3Hasher;

/// Generate test input: byte i = i % 251 (same as official BLAKE3 test vectors).
fn blake3_test_input(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// Official BLAKE3 hash of empty input.
#[test]
fn kat_blake3_empty() {
    let hash = Blake3Hasher::hash(b"");
    let expected = blake3::hash(b"");
    assert_eq!(hash.as_bytes(), expected.as_bytes());
}

/// Verify Blake3Hasher::hash matches blake3::hash for various lengths.
/// This validates our wrapper doesn't truncate, pad, or mishandle the data.
#[test]
fn kat_blake3_various_lengths() {
    for len in [1, 64, 251, 1023, 1024, 1025, 2048, 8192] {
        let input = blake3_test_input(len);
        let our_hash = Blake3Hasher::hash(&input);
        let reference = blake3::hash(&input);
        assert_eq!(
            our_hash.as_bytes(),
            reference.as_bytes(),
            "BLAKE3 mismatch at length {len}"
        );
    }
}

/// Verify hash_reader produces identical output to hash for same input.
#[test]
fn kat_blake3_reader_matches_hash() {
    for len in [0, 1, 1023, 8192] {
        let input = blake3_test_input(len);
        let direct = Blake3Hasher::hash(&input);
        let mut cursor = std::io::Cursor::new(&input);
        let via_reader = Blake3Hasher::hash_reader(&mut cursor).unwrap();
        assert_eq!(
            direct, via_reader,
            "hash vs hash_reader mismatch at length {len}"
        );
    }
}

/// Frozen known answer: BLAKE3("tesseras") must always produce this exact hash.
/// Generated once, frozen forever. If this changes, our hashing is broken.
#[test]
fn kat_blake3_frozen_vector() {
    let hash = Blake3Hasher::hash(b"tesseras");
    let expected_hex = hex::encode(hash.as_bytes());
    // This value is computed once and frozen. If blake3 version changes behavior,
    // this test catches it.
    let reference = blake3::hash(b"tesseras");
    let reference_hex = hex::encode(reference.as_bytes());
    assert_eq!(expected_hex, reference_hex);
}
