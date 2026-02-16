use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct ManifestJson {
    creator_pubkey: CreatorPubkey,
    signature_files: SignatureFiles,
    files: Vec<FileEntry>,
}

#[derive(Serialize)]
struct CreatorPubkey {
    ed25519: String,
    ml_dsa: Option<String>,
}

#[derive(Serialize)]
struct SignatureFiles {
    ed25519: String,
    ml_dsa: Option<String>,
}

#[derive(Serialize)]
struct FileEntry {
    path: String,
    hash: String,
    size: u64,
    mime: String,
}

/// Parse a plain-text MANIFEST (UTF-8 encoded).
/// Returns JSON string with creator pubkey, file entries, and signature file refs.
#[wasm_bindgen]
pub fn parse_manifest(data: &[u8]) -> Result<String, JsError> {
    parse_manifest_inner(data).map_err(|e| JsError::new(&e))
}

pub fn parse_manifest_inner(data: &[u8]) -> Result<String, String> {
    let text =
        std::str::from_utf8(data).map_err(|e| format!("MANIFEST is not valid UTF-8: {e}"))?;

    let manifest = tesseras_core::manifest::Manifest::parse(text)
        .map_err(|e| format!("failed to parse MANIFEST: {e}"))?;

    let json = ManifestJson {
        creator_pubkey: CreatorPubkey {
            ed25519: manifest.creator.clone(),
            ml_dsa: None,
        },
        signature_files: SignatureFiles {
            ed25519: "identity/signature.ed25519.sig".to_string(),
            ml_dsa: None,
        },
        files: manifest
            .entries
            .iter()
            .map(|e| FileEntry {
                path: e.path.clone(),
                hash: e.hash.to_string(),
                size: e.size,
                mime: e.mime_type.clone(),
            })
            .collect(),
    };

    serde_json::to_string(&json).map_err(|e| format!("JSON serialization failed: {e}"))
}

/// BLAKE3 hash of arbitrary bytes. Returns hex string.
#[wasm_bindgen]
pub fn hash_blake3(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Verify Ed25519 signature. pubkey: 32 bytes, signature: 64 bytes.
#[wasm_bindgen]
pub fn verify_ed25519(message: &[u8], signature: &[u8], pubkey: &[u8]) -> Result<bool, JsError> {
    verify_ed25519_inner(message, signature, pubkey).map_err(|e| JsError::new(&e))
}

pub fn verify_ed25519_inner(
    message: &[u8],
    signature: &[u8],
    pubkey: &[u8],
) -> Result<bool, String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let pubkey: [u8; 32] = pubkey
        .try_into()
        .map_err(|_| "Ed25519 public key must be 32 bytes".to_string())?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey)
        .map_err(|e| format!("invalid Ed25519 public key: {e}"))?;

    let sig = Signature::from_bytes(
        signature
            .try_into()
            .map_err(|_| "Ed25519 signature must be 64 bytes".to_string())?,
    );

    Ok(verifying_key.verify(message, &sig).is_ok())
}

/// Verify ML-DSA (FIPS 204) signature. Pure Rust.
///
/// ML-DSA verification not yet available in WASM.
/// Pure Rust ml-dsa crate is pre-release and tesseras-crypto does not
/// implement ML-DSA signing yet. Ed25519 verification is sufficient for now.
#[wasm_bindgen]
pub fn verify_ml_dsa(_message: &[u8], _signature: &[u8], _pubkey: &[u8]) -> Result<bool, JsError> {
    Err(JsError::new(
        "ML-DSA verification not available in WASM build",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- hash_blake3 tests ---

    #[test]
    fn hash_blake3_empty() {
        let hash = hash_blake3(&[]);
        assert_eq!(
            hash,
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn hash_blake3_hello() {
        let hash = hash_blake3(b"hello");
        let expected = blake3::hash(b"hello").to_hex().to_string();
        assert_eq!(hash, expected);
    }

    // --- verify_ed25519 tests ---

    #[test]
    fn verify_ed25519_valid_signature() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let message = b"TESSERA MANIFEST v1\ncreated: 2026-02-15\n";
        let signature = signing_key.sign(message);

        let result = verify_ed25519_inner(
            message,
            &signature.to_bytes(),
            signing_key.verifying_key().as_bytes(),
        );
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn verify_ed25519_invalid_signature() {
        use ed25519_dalek::SigningKey;

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let message = b"TESSERA MANIFEST v1\ncreated: 2026-02-15\n";
        let wrong_sig = [0u8; 64];

        let result =
            verify_ed25519_inner(message, &wrong_sig, signing_key.verifying_key().as_bytes());
        assert!(result.is_err() || matches!(result, Ok(false)));
    }

    #[test]
    fn verify_ed25519_wrong_key() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let wrong_key = SigningKey::from_bytes(&[99u8; 32]);
        let message = b"test message";
        let signature = signing_key.sign(message);

        let result = verify_ed25519_inner(
            message,
            &signature.to_bytes(),
            wrong_key.verifying_key().as_bytes(),
        );
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn verify_ed25519_bad_pubkey_length() {
        let result = verify_ed25519_inner(b"msg", &[0u8; 64], &[0u8; 16]);
        assert!(result.is_err());
    }

    // --- parse_manifest tests ---

    #[test]
    fn parse_manifest_valid() {
        let hash = "ab".repeat(32);
        let creator = "b3a7f2".to_string() + &"00".repeat(29);
        let manifest_text = format!(
            "TESSERA MANIFEST v1\n\
             created: 2026-02-15T00:00:00Z\n\
             creator: {creator}\n\
             content_hash: {hash}\n\
             encoding: UTF-8\n\
             schema: v1\n\
             \n\
             FILES:\n\
             memories/abc123/media.jpg  blake3:{hash}  image/jpeg  1024\n"
        );

        let result = parse_manifest_inner(manifest_text.as_bytes());
        assert!(result.is_ok(), "parse_manifest failed: {:?}", result.err());

        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["creator_pubkey"]["ed25519"], creator);
        assert!(json["files"].is_array());
        assert_eq!(json["files"][0]["path"], "memories/abc123/media.jpg");
        assert_eq!(json["files"][0]["size"], 1024);
        assert_eq!(json["files"][0]["mime"], "image/jpeg");
        assert_eq!(json["files"][0]["hash"], hash);
    }

    #[test]
    fn parse_manifest_invalid_utf8() {
        let result = parse_manifest_inner(&[0xFF, 0xFE, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_manifest_garbage() {
        let result = parse_manifest_inner(b"this is not a manifest");
        assert!(result.is_err());
    }
}
