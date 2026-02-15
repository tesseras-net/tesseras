+++
title = "Phase 4: Verify Without Installing Anything"
date = 2026-02-15T20:00:00+00:00
description = "Tesseras now compiles to WebAssembly — anyone can verify a tessera's integrity and authenticity directly in the browser, with no software to install."
+++

Trust shouldn't require installing software. If someone sends you a tessera — a bundle of preserved memories — you should be able to verify it's genuine and unmodified without downloading an app, creating an account, or trusting a server. That's what `tesseras-wasm` delivers: drag a tessera archive into a web page, and cryptographic verification happens entirely in your browser.

## What was built

**tesseras-wasm** — A Rust crate that compiles to WebAssembly via wasm-pack, exposing four stateless functions to JavaScript. The crate depends on `tesseras-core` for manifest parsing and calls cryptographic primitives directly (blake3, ed25519-dalek) rather than depending on `tesseras-crypto`, which pulls in C-based post-quantum libraries that don't compile to `wasm32-unknown-unknown`.

`parse_manifest` takes raw MANIFEST bytes (UTF-8 plain text, not MessagePack), delegates to `tesseras_core::manifest::Manifest::parse()`, and returns a JSON string with the creator's Ed25519 public key, signature file paths, and a list of files with their expected BLAKE3 hashes, sizes, and MIME types. Internal structs (`ManifestJson`, `CreatorPubkey`, `SignatureFiles`, `FileEntry`) are serialized with serde_json. The ML-DSA public key and signature file fields are present in the JSON contract but set to `null` — ready for when post-quantum signing is implemented on the native side.

`hash_blake3` computes a BLAKE3 hash of arbitrary bytes and returns a 64-character hex string. It's called once per file in the tessera to verify integrity against the MANIFEST.

`verify_ed25519` takes a message, a 64-byte signature, and a 32-byte public key, constructs an `ed25519_dalek::VerifyingKey`, and returns whether the signature is valid. Length validation returns descriptive errors ("Ed25519 public key must be 32 bytes") rather than panicking.

`verify_ml_dsa` is a stub that returns an error explaining ML-DSA verification is not yet available. This is deliberate: the `ml-dsa` crate on crates.io is v0.1.0-rc.7 (pre-release), and `tesseras-crypto` uses `pqcrypto-dilithium` (C-based CRYSTALS-Dilithium) which is byte-incompatible with FIPS 204 ML-DSA. Both sides need to use the same pure Rust implementation before cross-verification works. Ed25519 verification is sufficient — every tessera is Ed25519-signed.

All four functions use a two-layer pattern for testability: inner functions return `Result<T, String>` and are tested natively, while thin `#[wasm_bindgen]` wrappers convert errors to `JsError`. This avoids `JsError::new()` panicking on non-WASM targets during testing.

The compiled WASM binary is 109 KB raw and 44 KB gzipped — well under the 200 KB budget. wasm-opt applies `-Oz` optimization after wasm-pack builds with `opt-level = "z"`, LTO, and single codegen unit.

**@tesseras/verify** — A TypeScript npm package (`crates/tesseras-wasm/js/`) that orchestrates browser-side verification. The public API is a single function:

```typescript
async function verifyTessera(
  archive: Uint8Array,
  onProgress?: (current: number, total: number, file: string) => void
): Promise<VerificationResult>
```

The `VerificationResult` type provides everything a UI needs: overall validity, tessera hash, creator public keys, signature status (valid/invalid/missing for both Ed25519 and ML-DSA), per-file integrity results with expected and actual hashes, a list of unexpected files not in the MANIFEST, and an errors array.

Archive unpacking (`unpack.ts`) handles three formats: gzip-compressed tar (detected by `\x1f\x8b` magic bytes, decompressed with fflate then parsed as tar), ZIP (`PK\x03\x04` magic, unpacked with fflate's `unzipSync`), and raw tar (`ustar` at offset 257). A `normalizePath` function strips the leading `tessera-<hash>/` prefix so internal paths match MANIFEST entries.

Verification runs in a Web Worker (`worker.ts`) to keep the UI thread responsive. The worker initializes the WASM module, unpacks the archive, parses the MANIFEST, verifies the Ed25519 signature against the creator's public key, then hashes each file with BLAKE3 and compares against expected values. Progress messages stream back to the main thread after each file. If any signature is invalid, verification stops early without hashing files — failing fast on the most critical check.

The archive is transferred to the worker with zero-copy (`worker.postMessage({ type: "verify", archive }, [archive.buffer])`) to avoid duplicating potentially large tessera files in memory.

**Build pipeline** — Three new justfile targets: `wasm-build` runs wasm-pack with `--target web --release` and optimizes with wasm-opt; `wasm-size` reports raw and gzipped binary size; `test-wasm` runs the native test suite.

**Tests** — 9 native unit tests cover BLAKE3 hashing (empty input, known value), Ed25519 verification (valid signature, invalid signature, wrong key, bad key length), and MANIFEST parsing (valid manifest, invalid UTF-8, garbage input). 3 WASM integration tests run in headless Chrome via `wasm-pack test --headless --chrome`, verifying that `hash_blake3`, `verify_ed25519`, and `parse_manifest` work correctly when compiled to `wasm32-unknown-unknown`.

## Architecture decisions

- **No tesseras-crypto dependency**: the WASM crate calls blake3 and ed25519-dalek directly. `tesseras-crypto` depends on `pqcrypto-kyber` (C-based ML-KEM via pqcrypto-traits) which requires a C compiler toolchain and doesn't target wasm32. By depending only on pure Rust crates, the WASM build has zero C dependencies and compiles cleanly to WebAssembly.
- **ML-DSA deferred, not faked**: rather than silently skipping post-quantum verification, the stub returns an explicit error. This ensures that if a tessera contains an ML-DSA signature, the verification result will report `ml_dsa: "missing"` rather than pretending it was checked. The JS orchestrator handles this gracefully — a tessera is valid if Ed25519 passes and ML-DSA is missing (not yet implemented on either side).
- **Inner function pattern**: `JsError` cannot be constructed on non-WASM targets (it panics). Splitting each function into `foo_inner() -> Result<T, String>` and `foo() -> Result<T, JsError>` lets the native test suite exercise all logic without touching JavaScript types. The WASM integration tests in headless Chrome test the full `#[wasm_bindgen]` surface.
- **Web Worker isolation**: cryptographic operations (especially BLAKE3 over large media files) can take hundreds of milliseconds. Running in a Worker prevents UI jank. The streaming progress protocol (`{ type: "progress", current, total, file }`) lets the UI show a progress bar during verification of tesseras with many files.
- **Zero-copy transfer**: `archive.buffer` is transferred to the Worker, not copied. For a 50 MB tessera archive, this avoids doubling memory usage during verification.
- **Plain text MANIFEST, not MessagePack**: the WASM crate parses the same plain-text MANIFEST format as the CLI. This is by design — the MANIFEST is the tessera's Rosetta Stone, readable by anyone with a text editor. The `rmp-serde` dependency in the Cargo.toml is not used and will be removed.

## What comes next

- **Phase 4: Resilience and Scale** — OS packaging (Alpine, Arch, Debian, FreeBSD, OpenBSD), CI on SourceHut and GitHub Actions, security audits, browser-based tessera explorer at tesseras.net using @tesseras/verify
- **Phase 5: Exploration and Culture** — Public tessera browser by era/location/theme/language, institutional curation, genealogy integration, physical media export (M-DISC, microfilm, acid-free paper with QR)

Verification no longer requires trust in software. A tessera archive dropped into a browser is verified with the same cryptographic rigor as the CLI — same BLAKE3 hashes, same Ed25519 signatures, same MANIFEST parser. The difference is that now anyone can do it.
