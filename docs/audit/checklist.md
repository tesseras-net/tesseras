# Pre-Release Security Checklist

Complete before each tagged release of `tesseras-crypto`.

## Automated (CI covers these)

- [ ] `cargo audit` — no known CVEs
- [ ] `cargo deny check` — licenses, duplicates, bans all clean
- [ ] All KATs pass (`cargo test -p tesseras-crypto --test kat_tests --all-features`)
- [ ] All fuzz targets pass (`just audit-fuzz`)

## Manual (developer runs before tagging)

- [ ] `cargo-mutants` on `dual.rs` — zero surviving mutations in verification logic
- [ ] `cargo-mutants` on `sealed.rs` — zero surviving mutations in unseal/verify logic
- [ ] `mutants-report.txt` updated with:
  - cargo-mutants version
  - Commit hash
  - Date
  - Full results
- [ ] Nonce handling review: grep all AES-GCM nonce generation sites, confirm each uses `OsRng`
- [ ] Key material lifecycle: all types holding secrets have `Drop` impl with `zeroize()`
  - `Ed25519KeyPair` in `ed25519.rs` (SigningKey has internal ZeroizeOnDrop)
  - `HybridKeyPair` in `kem.rs`
  - `HeirShare` in `shamir/mod.rs`
  - `KeyMaterial` in `tesseras-core/ports.rs`
- [ ] Error oracle review: crypto error variants don't leak timing or content info
  - `CryptoError::DecryptFailed` — generic, no details (correct)
  - `CryptoError::VerificationFailed` — generic, no details (correct)
- [ ] Dependency diff: review new transitive deps since last release (`cargo tree --depth 1 -i`)

## Reviewer Sign-Off

| Release | Date | Reviewer | Notes |
|---------|------|----------|-------|
|         |      |          |       |
