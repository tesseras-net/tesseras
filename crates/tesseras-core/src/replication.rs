use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::types::ContentHash;

/// Maximum tessera size: 10 GiB.
pub const MAX_TESSERA_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// 4 MB boundary between Small and Medium tiers.
const SMALL_MEDIUM_BOUNDARY: u64 = 4 * 1024 * 1024;

/// 256 MB boundary between Medium and Large tiers.
const MEDIUM_LARGE_BOUNDARY: u64 = 256 * 1024 * 1024;

/// Erasure coding tier based on tessera size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FragmentationTier {
    /// Tesseras < 4 MB: replicate whole file, no erasure coding.
    Small { replication_factor: u8 },
    /// Tesseras 4 MB..256 MB: 16 data + 8 parity shards.
    Medium {
        data_shards: u16,
        parity_shards: u16,
        fragment_size: u64,
        replication_factor: u8,
    },
    /// Tesseras >= 256 MB: 48 data + 24 parity shards.
    Large {
        data_shards: u16,
        parity_shards: u16,
        fragment_size: u64,
        replication_factor: u8,
    },
}

/// Plan describing how a tessera should be fragmented and replicated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentPlan {
    pub tier: FragmentationTier,
    pub tessera_hash: ContentHash,
    pub tessera_size: u64,
}

impl FragmentPlan {
    /// Select fragmentation tier based on tessera size.
    pub fn new(tessera_hash: ContentHash, tessera_size: u64) -> Result<Self, CoreError> {
        if tessera_size > MAX_TESSERA_SIZE {
            return Err(CoreError::TesseraTooBig {
                size: tessera_size,
                max: MAX_TESSERA_SIZE,
            });
        }

        let tier = if tessera_size < SMALL_MEDIUM_BOUNDARY {
            FragmentationTier::Small {
                replication_factor: 7,
            }
        } else if tessera_size < MEDIUM_LARGE_BOUNDARY {
            FragmentationTier::Medium {
                data_shards: 16,
                parity_shards: 8,
                fragment_size: tessera_size / 16,
                replication_factor: 7,
            }
        } else {
            FragmentationTier::Large {
                data_shards: 48,
                parity_shards: 24,
                fragment_size: tessera_size / 48,
                replication_factor: 7,
            }
        };

        Ok(Self {
            tier,
            tessera_hash,
            tessera_size,
        })
    }
}

/// Unique identifier for a fragment within a tessera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentId {
    pub tessera_hash: ContentHash,
    pub index: u16,
    pub is_parity: bool,
    pub checksum: ContentHash,
}

impl FragmentId {
    pub fn new(
        tessera_hash: ContentHash,
        index: u16,
        data_shards: u16,
        checksum: ContentHash,
    ) -> Self {
        Self {
            tessera_hash,
            index,
            is_parity: index >= data_shards,
            checksum,
        }
    }
}

/// Single entry in an attestation: fragment index + its BLAKE3 checksum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationEntry {
    pub fragment_index: u16,
    pub checksum: ContentHash,
}

/// Proof that a node holds specific fragments of a tessera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub tessera_hash: ContentHash,
    pub entries: Vec<AttestationEntry>,
    pub timestamp: DateTime<Utc>,
    pub signature: Vec<u8>,
}

/// Acknowledgement of a REPLICATE request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicateAck {
    pub accepted: bool,
    pub fragments_held: Vec<u16>,
}

/// Envelope wrapping a fragment with metadata for transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentEnvelope {
    pub id: FragmentId,
    pub plan: FragmentPlan,
    pub original_tessera_size: u64,
    pub fragment_size: u64,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContentHash;

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    #[test]
    fn plan_small_tier() {
        let plan = FragmentPlan::new(hash(0x01), 1_000_000).unwrap(); // 1 MB
        assert!(matches!(
            plan.tier,
            FragmentationTier::Small {
                replication_factor: 7
            }
        ));
    }

    #[test]
    fn plan_medium_tier() {
        let plan = FragmentPlan::new(hash(0x02), 100_000_000).unwrap(); // 100 MB
        match plan.tier {
            FragmentationTier::Medium {
                data_shards: 16,
                parity_shards: 8,
                fragment_size,
                replication_factor: 7,
            } => {
                assert_eq!(fragment_size, 100_000_000 / 16);
            }
            _ => panic!("expected Medium tier"),
        }
    }

    #[test]
    fn plan_large_tier() {
        let plan = FragmentPlan::new(hash(0x03), 3_000_000_000).unwrap(); // 3 GB
        match plan.tier {
            FragmentationTier::Large {
                data_shards: 48,
                parity_shards: 24,
                fragment_size,
                replication_factor: 7,
            } => {
                assert_eq!(fragment_size, 3_000_000_000 / 48);
            }
            _ => panic!("expected Large tier"),
        }
    }

    #[test]
    fn plan_rejects_oversized() {
        let result = FragmentPlan::new(hash(0x04), MAX_TESSERA_SIZE + 1);
        assert!(result.is_err());
    }

    #[test]
    fn plan_boundary_small_medium() {
        // Exactly 4MB should be Medium
        let plan = FragmentPlan::new(hash(0x05), 4 * 1024 * 1024).unwrap();
        assert!(matches!(plan.tier, FragmentationTier::Medium { .. }));
        // Just under 4MB should be Small
        let plan = FragmentPlan::new(hash(0x06), 4 * 1024 * 1024 - 1).unwrap();
        assert!(matches!(plan.tier, FragmentationTier::Small { .. }));
    }

    #[test]
    fn plan_boundary_medium_large() {
        // Exactly 256MB should be Large
        let plan = FragmentPlan::new(hash(0x07), 256 * 1024 * 1024).unwrap();
        assert!(matches!(plan.tier, FragmentationTier::Large { .. }));
        // Just under 256MB should be Medium
        let plan = FragmentPlan::new(hash(0x08), 256 * 1024 * 1024 - 1).unwrap();
        assert!(matches!(plan.tier, FragmentationTier::Medium { .. }));
    }

    #[test]
    fn fragment_id_parity_flag() {
        let id = FragmentId::new(hash(0x01), 15, 16, hash(0xaa));
        assert!(!id.is_parity); // index 15 < data_shards 16
        let id = FragmentId::new(hash(0x01), 16, 16, hash(0xbb));
        assert!(id.is_parity); // index 16 >= data_shards 16
    }

    #[test]
    fn serde_roundtrip_fragment_plan() {
        let plan = FragmentPlan::new(hash(0x01), 100_000_000).unwrap();
        let bytes = rmp_serde::to_vec(&plan).unwrap();
        let decoded: FragmentPlan = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded, plan);
    }

    #[test]
    fn serde_roundtrip_attestation() {
        let att = Attestation {
            tessera_hash: hash(0x01),
            entries: vec![AttestationEntry {
                fragment_index: 0,
                checksum: hash(0xaa),
            }],
            timestamp: chrono::Utc::now(),
            signature: vec![0xde, 0xad],
        };
        let bytes = rmp_serde::to_vec(&att).unwrap();
        let decoded: Attestation = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded, att);
    }
}
