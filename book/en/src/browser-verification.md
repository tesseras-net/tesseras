# Browser Verification

Tesseras can be verified directly in a web browser without installing any software. The `@tesseras/verify` package runs cryptographic verification entirely client-side using WebAssembly.

## How it works

When you drop a tessera archive into a verification page:

1. The archive (`.tar.gz`, `.zip`, or `.tar`) is unpacked in the browser
2. The MANIFEST is parsed to extract the creator's public key, file list, and expected hashes
3. The Ed25519 signature is verified against the creator's public key
4. Each file's BLAKE3 hash is computed and compared to the MANIFEST
5. A detailed result shows which files are intact and whether signatures are valid

All of this happens in a Web Worker to keep the page responsive. Progress updates stream back to the UI as each file is verified.

## Verification result

The result includes:

| Field | Description |
|-------|-------------|
| `valid` | Overall pass/fail |
| `tessera_hash` | BLAKE3 hash of the MANIFEST |
| `signatures.ed25519` | `valid`, `invalid`, or `missing` |
| `signatures.ml_dsa` | `valid`, `invalid`, or `missing` (currently always `missing`) |
| `files` | Per-file status with expected and actual hashes |
| `unexpected_files` | Files in the archive not listed in the MANIFEST |
| `errors` | Any errors encountered during verification |

## What is verified

- **Signature authenticity** — the MANIFEST was signed by the creator's Ed25519 private key
- **File integrity** — every file in the MANIFEST has the correct BLAKE3 hash
- **Completeness** — all files listed in the MANIFEST are present in the archive
- **No extras** — files not in the MANIFEST are flagged as unexpected

## What is NOT verified

- **Identity** — browser verification confirms the tessera was signed by a specific key, but doesn't tell you who owns that key. You need an out-of-band way to confirm the creator's public key.
- **ML-DSA (post-quantum)** — post-quantum signature verification is not yet available in the browser. Ed25519 signatures are verified.

## Using the npm package

For developers building verification into their own applications:

```typescript
import { verifyTessera } from "@tesseras/verify";

const archive = new Uint8Array(/* tessera archive bytes */);

const result = await verifyTessera(archive, (current, total, file) => {
  console.log(`Verifying ${file} (${current}/${total})`);
});

if (result.valid) {
  console.log("Tessera is authentic and intact");
} else {
  console.log("Verification failed:", result.errors);
}
```

## Comparison with CLI verification

| Feature | `tes verify` (CLI) | Browser verification |
|---------|-------------------|---------------------|
| Ed25519 signatures | Yes | Yes |
| ML-DSA signatures | Yes (when available) | Not yet |
| BLAKE3 file hashes | Yes | Yes |
| Requires installation | Yes | No |
| Works offline | Yes | Yes (after page loads) |
| Large files | No limit | Limited by browser memory |
| WASM binary size | N/A | 44 KB gzipped |

## Technical details

The WASM binary is compiled from Rust using `wasm-pack`. It includes:

- `blake3` — for file integrity hashing
- `ed25519-dalek` — for signature verification
- `tesseras-core` — for MANIFEST parsing

The binary is 109 KB raw (44 KB gzipped). It does not include `tesseras-crypto` or any C dependencies — all cryptographic operations use pure Rust implementations.
