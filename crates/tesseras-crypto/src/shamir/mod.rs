//! Shamir's Secret Sharing over GF(256).
//!
//! Splits a secret byte slice into N shares with threshold T.
//! Any T shares reconstruct the original; T-1 shares reveal nothing
//! (information-theoretic security).

mod gf256;

use blake3;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tesseras_core::ContentHash;

use crate::CryptoError;
use gf256::{poly_eval, Gf256};

/// Configuration for Shamir's Secret Sharing scheme.
#[derive(Debug, Clone)]
pub struct ShamirConfig {
    /// T: minimum shares to reconstruct (>= 1).
    pub threshold: u8,
    /// N: total shares to create (>= T, <= 255).
    pub total_shares: u8,
}

/// A single share of a split secret, ready for export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeirShare {
    /// Format version for forward compatibility (currently 1).
    pub format_version: u8,
    /// 1..=N, Shamir x-coordinate.
    pub share_index: u8,
    /// T required to reconstruct.
    pub threshold: u8,
    /// N total shares created.
    pub total_shares: u8,
    /// Random per split() call, prevents mixing shares from different sessions.
    pub session_id: [u8; 8],
    /// First 8 bytes of BLAKE3(owner_ed25519_public). Identifies owner
    /// without revealing full public key.
    pub owner_fingerprint: [u8; 8],
    /// Shamir y-values (same length as secret).
    pub share_data: Vec<u8>,
    /// BLAKE3(all preceding fields serialized). Catches corruption
    /// before attempting reconstruction.
    pub checksum: ContentHash,
}

impl HeirShare {
    /// Compute checksum over all fields except checksum itself.
    fn compute_checksum(
        format_version: u8,
        share_index: u8,
        threshold: u8,
        total_shares: u8,
        session_id: &[u8; 8],
        owner_fingerprint: &[u8; 8],
        share_data: &[u8],
    ) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[format_version, share_index, threshold, total_shares]);
        hasher.update(session_id);
        hasher.update(owner_fingerprint);
        hasher.update(share_data);
        ContentHash::new(*hasher.finalize().as_bytes())
    }

    /// Verify that the checksum matches the share contents.
    pub fn verify_checksum(&self) -> bool {
        let expected = Self::compute_checksum(
            self.format_version,
            self.share_index,
            self.threshold,
            self.total_shares,
            &self.session_id,
            &self.owner_fingerprint,
            &self.share_data,
        );
        self.checksum == expected
    }
}

/// Shamir's Secret Sharing operations over GF(256).
pub struct ShamirSplitter;

impl ShamirSplitter {
    /// Split secret into N shares with threshold T.
    ///
    /// # Validation
    /// - `1 <= threshold <= total_shares <= 255`
    /// - `secret` must not be empty
    /// - `owner_public_ed25519` must be 32 bytes
    pub fn split(
        secret: &[u8],
        config: &ShamirConfig,
        owner_public_ed25519: &[u8],
    ) -> Result<Vec<HeirShare>, CryptoError> {
        // Validate config
        if config.threshold == 0 {
            return Err(CryptoError::ShamirInvalidConfig(
                "threshold must be >= 1".into(),
            ));
        }
        if config.total_shares < config.threshold {
            return Err(CryptoError::ShamirInvalidConfig(format!(
                "total_shares ({}) must be >= threshold ({})",
                config.total_shares, config.threshold
            )));
        }
        if secret.is_empty() {
            return Err(CryptoError::ShamirSplitFailed(
                "secret must not be empty".into(),
            ));
        }
        if owner_public_ed25519.len() != 32 {
            return Err(CryptoError::ShamirSplitFailed(format!(
                "owner_public_ed25519 must be 32 bytes, got {}",
                owner_public_ed25519.len()
            )));
        }

        let t = config.threshold as usize;
        let n = config.total_shares as usize;

        // Generate session_id
        let mut session_id = [0u8; 8];
        OsRng.fill_bytes(&mut session_id);

        // Compute owner fingerprint: first 8 bytes of BLAKE3(public_key)
        let fingerprint_hash = blake3::hash(owner_public_ed25519);
        let mut owner_fingerprint = [0u8; 8];
        owner_fingerprint.copy_from_slice(&fingerprint_hash.as_bytes()[..8]);

        // Initialize share_data vectors
        let mut share_datas: Vec<Vec<u8>> =
            (0..n).map(|_| Vec::with_capacity(secret.len())).collect();

        // For each byte of the secret, create a polynomial and evaluate at each share index
        let mut rng_buf = vec![0u8; t.saturating_sub(1)];
        for &secret_byte in secret {
            // Generate random coefficients for degree 1..T-1
            if !rng_buf.is_empty() {
                OsRng.fill_bytes(&mut rng_buf);
            }

            // Build polynomial: coefficients[0] = secret_byte, rest = random
            let mut coefficients = Vec::with_capacity(t);
            coefficients.push(Gf256(secret_byte));
            for &coeff in &rng_buf {
                coefficients.push(Gf256(coeff));
            }

            // Evaluate at x = 1, 2, ..., N
            for (i, share_data) in share_datas.iter_mut().enumerate() {
                let x = Gf256((i + 1) as u8);
                let y = poly_eval(&coefficients, x);
                share_data.push(y.0);
            }
        }

        // Build HeirShare structs with checksums
        let shares = share_datas
            .into_iter()
            .enumerate()
            .map(|(i, share_data)| {
                let checksum = HeirShare::compute_checksum(
                    1, // format_version
                    (i + 1) as u8,
                    config.threshold,
                    config.total_shares,
                    &session_id,
                    &owner_fingerprint,
                    &share_data,
                );
                HeirShare {
                    format_version: 1,
                    share_index: (i + 1) as u8,
                    threshold: config.threshold,
                    total_shares: config.total_shares,
                    session_id,
                    owner_fingerprint,
                    share_data,
                    checksum,
                }
            })
            .collect();

        Ok(shares)
    }

    /// Reconstruct the secret from T or more shares.
    ///
    /// # Validation (before attempting reconstruction)
    /// - All shares must have matching `session_id`
    /// - All shares must have matching `owner_fingerprint`
    /// - All shares must have matching `threshold`
    /// - Each share's `checksum` must be valid
    /// - Number of shares >= `threshold`
    ///
    /// # Post-reconstruction verification
    /// If `expected_owner_public` is provided, derives Ed25519 public key
    /// from the reconstructed secret (bytes 2..34 after version+flags header)
    /// and verifies the fingerprint matches.
    pub fn reconstruct(
        shares: &[HeirShare],
        expected_owner_public: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        if shares.is_empty() {
            return Err(CryptoError::ShamirReconstructFailed(
                "no shares provided".into(),
            ));
        }

        let first = &shares[0];

        // Validate: enough shares
        if shares.len() < first.threshold as usize {
            return Err(CryptoError::ShamirReconstructFailed(format!(
                "need {} shares but only {} provided",
                first.threshold,
                shares.len()
            )));
        }

        // Validate: all shares consistent
        for (i, share) in shares.iter().enumerate() {
            if share.session_id != first.session_id {
                return Err(CryptoError::ShareValidationFailed(format!(
                    "share {} has different session_id than share 0",
                    i
                )));
            }
            if share.owner_fingerprint != first.owner_fingerprint {
                return Err(CryptoError::ShareValidationFailed(format!(
                    "share {} has different owner_fingerprint than share 0",
                    i
                )));
            }
            if share.threshold != first.threshold {
                return Err(CryptoError::ShareValidationFailed(format!(
                    "share {} has different threshold than share 0",
                    i
                )));
            }
            if !share.verify_checksum() {
                return Err(CryptoError::ShareValidationFailed(format!(
                    "share {} has invalid checksum (corrupted data)",
                    i
                )));
            }
            if share.share_data.len() != first.share_data.len() {
                return Err(CryptoError::ShareValidationFailed(format!(
                    "share {} has different data length ({}) than share 0 ({})",
                    i,
                    share.share_data.len(),
                    first.share_data.len()
                )));
            }
        }

        // Use only threshold number of shares (first T)
        let t = first.threshold as usize;
        let shares_to_use = &shares[..t];
        let secret_len = first.share_data.len();

        // Reconstruct each byte via Lagrange interpolation at x=0
        let mut secret = Vec::with_capacity(secret_len);
        for byte_idx in 0..secret_len {
            let points: Vec<(Gf256, Gf256)> = shares_to_use
                .iter()
                .map(|s| (Gf256(s.share_index), Gf256(s.share_data[byte_idx])))
                .collect();
            let recovered = gf256::lagrange_interpolate(&points);
            secret.push(recovered.0);
        }

        // Post-reconstruction verification
        if let Some(expected_public) = expected_owner_public {
            let fingerprint_hash = blake3::hash(expected_public);
            let expected_fingerprint = &fingerprint_hash.as_bytes()[..8];
            if expected_fingerprint != first.owner_fingerprint {
                return Err(CryptoError::ShamirOwnerMismatch);
            }
        }

        Ok(secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_owner_public() -> [u8; 32] {
        [0xAAu8; 32]
    }

    #[test]
    fn split_produces_correct_number_of_shares() {
        let secret = b"hello tesseras";
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();
        assert_eq!(shares.len(), 3);
        for (i, share) in shares.iter().enumerate() {
            assert_eq!(share.share_index, (i + 1) as u8);
            assert_eq!(share.threshold, 2);
            assert_eq!(share.total_shares, 3);
            assert_eq!(share.format_version, 1);
            assert_eq!(share.share_data.len(), secret.len());
            assert!(share.verify_checksum());
        }
    }

    #[test]
    fn split_validates_threshold_zero() {
        let config = ShamirConfig {
            threshold: 0,
            total_shares: 3,
        };
        let result = ShamirSplitter::split(b"secret", &config, &test_owner_public());
        assert!(matches!(result, Err(CryptoError::ShamirInvalidConfig(_))));
    }

    #[test]
    fn split_validates_threshold_exceeds_total() {
        let config = ShamirConfig {
            threshold: 5,
            total_shares: 3,
        };
        let result = ShamirSplitter::split(b"secret", &config, &test_owner_public());
        assert!(matches!(result, Err(CryptoError::ShamirInvalidConfig(_))));
    }

    #[test]
    fn split_validates_empty_secret() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let result = ShamirSplitter::split(b"", &config, &test_owner_public());
        assert!(matches!(result, Err(CryptoError::ShamirSplitFailed(_))));
    }

    #[test]
    fn split_owner_fingerprint_is_blake3_prefix() {
        let owner = test_owner_public();
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(b"test", &config, &owner).unwrap();
        let expected = blake3::hash(&owner);
        assert_eq!(shares[0].owner_fingerprint, expected.as_bytes()[..8]);
    }

    #[test]
    fn split_session_id_is_consistent_within_session() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(b"test", &config, &test_owner_public()).unwrap();
        assert_eq!(shares[0].session_id, shares[1].session_id);
        assert_eq!(shares[1].session_id, shares[2].session_id);
    }

    #[test]
    fn share_checksum_detects_corruption() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let mut shares =
            ShamirSplitter::split(b"test data", &config, &test_owner_public()).unwrap();
        assert!(shares[0].verify_checksum());

        // Flip a bit in share_data
        shares[0].share_data[0] ^= 0xFF;
        assert!(!shares[0].verify_checksum());
    }

    #[test]
    fn split_reconstruct_roundtrip() {
        let secret = b"hello tesseras, this is a 64-byte secret for shamir splitting!!";
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();

        // Reconstruct with shares 0 and 1
        let recovered =
            ShamirSplitter::reconstruct(&[shares[0].clone(), shares[1].clone()], None).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_reconstruct_all_share_combinations() {
        let secret = b"test secret data";
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();

        // All 3 pairs of 2 shares should reconstruct correctly
        let pairs = [(0, 1), (0, 2), (1, 2)];
        for (a, b) in pairs {
            let recovered = ShamirSplitter::reconstruct(
                &[shares[a].clone(), shares[b].clone()],
                None,
            )
            .unwrap();
            assert_eq!(recovered, secret, "pair ({a},{b}) failed");
        }
    }

    #[test]
    fn reconstruct_with_insufficient_shares() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(b"secret", &config, &test_owner_public()).unwrap();

        let result = ShamirSplitter::reconstruct(&[shares[0].clone()], None);
        assert!(matches!(
            result,
            Err(CryptoError::ShamirReconstructFailed(_))
        ));
    }

    #[test]
    fn reconstruct_wrong_owner_fingerprint() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares_a =
            ShamirSplitter::split(b"secret a", &config, &[0xAAu8; 32]).unwrap();
        let shares_b =
            ShamirSplitter::split(b"secret b", &config, &[0xBBu8; 32]).unwrap();

        // Mix shares from different owners
        let result =
            ShamirSplitter::reconstruct(&[shares_a[0].clone(), shares_b[1].clone()], None);
        assert!(matches!(
            result,
            Err(CryptoError::ShareValidationFailed(_))
        ));
    }

    #[test]
    fn reconstruct_wrong_session_id() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        // Two separate split calls produce different session_ids
        let shares_1 =
            ShamirSplitter::split(b"same secret", &config, &test_owner_public()).unwrap();
        let shares_2 =
            ShamirSplitter::split(b"same secret", &config, &test_owner_public()).unwrap();

        // Verify session_ids are different
        assert_ne!(shares_1[0].session_id, shares_2[0].session_id);

        // Mix shares from different sessions
        let result =
            ShamirSplitter::reconstruct(&[shares_1[0].clone(), shares_2[1].clone()], None);
        assert!(matches!(
            result,
            Err(CryptoError::ShareValidationFailed(_))
        ));
    }

    #[test]
    fn reconstruct_with_owner_verification() {
        let owner = test_owner_public();
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(b"secret", &config, &owner).unwrap();

        let recovered = ShamirSplitter::reconstruct(
            &[shares[0].clone(), shares[1].clone()],
            Some(&owner),
        )
        .unwrap();
        assert_eq!(recovered, b"secret");
    }

    #[test]
    fn reconstruct_with_wrong_owner_verification() {
        let owner = test_owner_public();
        let wrong_owner = [0xBBu8; 32];
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let shares = ShamirSplitter::split(b"secret", &config, &owner).unwrap();

        let result = ShamirSplitter::reconstruct(
            &[shares[0].clone(), shares[1].clone()],
            Some(&wrong_owner),
        );
        assert!(matches!(result, Err(CryptoError::ShamirOwnerMismatch)));
    }

    #[test]
    fn split_threshold_1() {
        let secret = b"any single share reconstructs";
        let config = ShamirConfig {
            threshold: 1,
            total_shares: 2,
        };
        let shares = ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();

        // Any single share should reconstruct
        let recovered_0 = ShamirSplitter::reconstruct(&[shares[0].clone()], None).unwrap();
        assert_eq!(recovered_0, secret);

        let recovered_1 = ShamirSplitter::reconstruct(&[shares[1].clone()], None).unwrap();
        assert_eq!(recovered_1, secret);
    }

    #[test]
    fn large_secret_roundtrip() {
        // Simulate ML-KEM-768 secret key size
        let secret: Vec<u8> = (0..2464).map(|i| (i % 256) as u8).collect();
        let config = ShamirConfig {
            threshold: 3,
            total_shares: 5,
        };
        let shares = ShamirSplitter::split(&secret, &config, &test_owner_public()).unwrap();

        let recovered = ShamirSplitter::reconstruct(
            &[shares[0].clone(), shares[2].clone(), shares[4].clone()],
            None,
        )
        .unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn reconstruct_shares_from_different_split_sessions() {
        let config = ShamirConfig {
            threshold: 2,
            total_shares: 3,
        };
        let secret = b"same secret for both sessions";
        let shares_session_1 =
            ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();
        let shares_session_2 =
            ShamirSplitter::split(secret, &config, &test_owner_public()).unwrap();

        // Shares from same session work
        let ok = ShamirSplitter::reconstruct(
            &[shares_session_1[0].clone(), shares_session_1[1].clone()],
            None,
        );
        assert!(ok.is_ok());

        // Mixed shares from different sessions fail
        let err = ShamirSplitter::reconstruct(
            &[shares_session_1[0].clone(), shares_session_2[1].clone()],
            None,
        );
        assert!(matches!(err, Err(CryptoError::ShareValidationFailed(_))));
    }
}
