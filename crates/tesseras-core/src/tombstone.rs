use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::ContentHash;

/// A signed retraction record. Creator publishes this to delete a memory
/// from the network. Tombstones are authoritative: a valid tombstone with
/// a timestamp newer than the original publish causes peers to reject any
/// STORE for that hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    pub hash: ContentHash,
    pub retracted_at: DateTime<Utc>,
    pub creator_pubkey: String,
    pub ed25519_signature: Vec<u8>,
    pub mldsa_signature: Vec<u8>,
}

impl Tombstone {
    /// Canonical bytes for signing: hash || retracted_at timestamp.
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32 + 8);
        buf.extend_from_slice(self.hash.as_bytes());
        buf.extend_from_slice(&self.retracted_at.timestamp().to_le_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tombstone_serde_roundtrip() {
        let t = Tombstone {
            hash: ContentHash::new([0xab; 32]),
            retracted_at: Utc::now(),
            creator_pubkey: "deadbeef".to_string(),
            ed25519_signature: vec![0x01; 64],
            mldsa_signature: vec![0x02; 128],
        };
        let bytes = rmp_serde::to_vec(&t).unwrap();
        let parsed: Tombstone = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn signable_bytes_deterministic() {
        let t = Tombstone {
            hash: ContentHash::new([0xab; 32]),
            retracted_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            creator_pubkey: "test".to_string(),
            ed25519_signature: vec![],
            mldsa_signature: vec![],
        };
        let bytes1 = t.signable_bytes();
        let bytes2 = t.signable_bytes();
        assert_eq!(bytes1, bytes2);
        assert_eq!(bytes1.len(), 40); // 32 hash + 8 timestamp
    }
}
