//! WASM integration tests — run with:
//! wasm-pack test crates/tesseras-wasm --headless --chrome

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn wasm_hash_blake3_works() {
    let hash = tesseras_wasm::hash_blake3(b"hello");
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // 32 bytes as hex = 64 chars
}

#[wasm_bindgen_test]
fn wasm_verify_ed25519_rejects_bad_key_length() {
    let result = tesseras_wasm::verify_ed25519(b"msg", &[0u8; 64], &[0u8; 16]);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn wasm_parse_manifest_rejects_garbage() {
    let result = tesseras_wasm::parse_manifest(b"not a manifest");
    assert!(result.is_err());
}
