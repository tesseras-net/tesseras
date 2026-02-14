use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::enums::Visibility;
use crate::types::{ContentHash, NodeId};

/// Capability bitfield exchanged during Pong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities(pub u64);

impl Capabilities {
    pub const PING: u64 = 1 << 0;
    pub const FIND_NODE: u64 = 1 << 1;
    pub const FIND_VALUE: u64 = 1 << 2;
    pub const STORE: u64 = 1 << 3;
    pub const REPLICATE: u64 = 1 << 4;
    pub const ATTEST: u64 = 1 << 5;
    pub const RELAY: u64 = 1 << 6;

    /// Phase 1 default: PING | FIND_NODE | FIND_VALUE | STORE
    pub fn phase1_default() -> Self {
        Self(Self::PING | Self::FIND_NODE | Self::FIND_VALUE | Self::STORE)
    }

    /// Phase 2 default: Phase 1 + REPLICATE | ATTEST
    pub fn phase2_default() -> Self {
        Self(
            Self::PING
                | Self::FIND_NODE
                | Self::FIND_VALUE
                | Self::STORE
                | Self::REPLICATE
                | Self::ATTEST,
        )
    }

    pub fn has(&self, cap: u64) -> bool {
        self.0 & cap != 0
    }
}

/// A node's identity: Ed25519 public key + PoW nonce = NodeId.
/// NodeId = BLAKE3(public_key || nonce)[..20], must have POW_DIFFICULTY leading zero bits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node_id: NodeId,
    pub public_key: [u8; 32],
    pub nonce: u64,
}

/// Information about a known peer in the routing table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub identity: NodeIdentity,
    pub addr: SocketAddr,
    /// Additional addresses (e.g. IPv6 when primary is IPv4). Empty for
    /// nodes that only have a single address.
    #[serde(default)]
    pub alt_addrs: Vec<SocketAddr>,
    pub capabilities: Capabilities,
}

impl NodeInfo {
    /// Iterate over all addresses (primary + alternates).
    pub fn all_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        std::iter::once(&self.addr).chain(self.alt_addrs.iter())
    }
}

/// A node that holds (part of) a tessera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolderInfo {
    pub node_id: NodeId,
    pub addr: SocketAddr,
    /// Additional addresses for this holder.
    #[serde(default)]
    pub alt_addrs: Vec<SocketAddr>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub fragments: Vec<u32>,
}

impl HolderInfo {
    /// Iterate over all addresses (primary + alternates).
    pub fn all_addrs(&self) -> impl Iterator<Item = &SocketAddr> {
        std::iter::once(&self.addr).chain(self.alt_addrs.iter())
    }
}

/// DHT stores lightweight pointers to tessera holders (not the data itself).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TesseraPointer {
    pub tessera_hash: ContentHash,
    pub size_bytes: u64,
    pub holders: Vec<HolderInfo>,
    pub visibility: Visibility,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_phase1_default_has_core_rpcs() {
        let caps = Capabilities::phase1_default();
        assert!(caps.has(Capabilities::PING));
        assert!(caps.has(Capabilities::FIND_NODE));
        assert!(caps.has(Capabilities::FIND_VALUE));
        assert!(caps.has(Capabilities::STORE));
    }

    #[test]
    fn capabilities_phase1_default_lacks_phase2() {
        let caps = Capabilities::phase1_default();
        assert!(!caps.has(Capabilities::REPLICATE));
        assert!(!caps.has(Capabilities::ATTEST));
        assert!(!caps.has(Capabilities::RELAY));
    }

    #[test]
    fn capabilities_phase2_default_has_replication() {
        let caps = Capabilities::phase2_default();
        assert!(caps.has(Capabilities::REPLICATE));
        assert!(caps.has(Capabilities::ATTEST));
        // Still has phase 1 caps
        assert!(caps.has(Capabilities::PING));
        assert!(caps.has(Capabilities::FIND_NODE));
    }

    #[test]
    fn capabilities_serde_roundtrip() {
        let caps = Capabilities::phase1_default();
        let bytes = rmp_serde::to_vec(&caps).unwrap();
        let parsed: Capabilities = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, caps);
    }

    #[test]
    fn node_identity_serde_roundtrip() {
        let id = NodeIdentity {
            node_id: NodeId::new([0xab; 20]),
            public_key: [0xcd; 32],
            nonce: 42,
        };
        let bytes = rmp_serde::to_vec(&id).unwrap();
        let parsed: NodeIdentity = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn tessera_pointer_serde_roundtrip() {
        let ptr = TesseraPointer {
            tessera_hash: ContentHash::new([0x01; 32]),
            size_bytes: 3_000_000,
            holders: vec![HolderInfo {
                node_id: NodeId::new([0x02; 20]),
                addr: "127.0.0.1:4433".parse().unwrap(),
                alt_addrs: vec![],
                last_seen: chrono::Utc::now(),
                fragments: vec![0, 1, 2],
            }],
            visibility: Visibility::Public,
            created_at: chrono::Utc::now(),
        };
        let bytes = rmp_serde::to_vec(&ptr).unwrap();
        let parsed: TesseraPointer = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed.tessera_hash, ptr.tessera_hash);
        assert_eq!(parsed.holders.len(), 1);
        assert_eq!(parsed.holders[0].fragments, vec![0, 1, 2]);
    }

    #[test]
    fn node_info_serde_roundtrip_ipv6() {
        let info = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new([0xab; 20]),
                public_key: [0xcd; 32],
                nonce: 99,
            },
            addr: "[::1]:4433".parse().unwrap(),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };
        let bytes = rmp_serde::to_vec(&info).unwrap();
        let parsed: NodeInfo = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, info);
        assert!(parsed.addr.is_ipv6());
    }

    #[test]
    fn node_info_serde_roundtrip() {
        let info = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new([0xab; 20]),
                public_key: [0xcd; 32],
                nonce: 99,
            },
            addr: "192.168.1.1:4433".parse().unwrap(),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };
        let bytes = rmp_serde::to_vec(&info).unwrap();
        let parsed: NodeInfo = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, info);
    }

    #[test]
    fn node_info_alt_addrs_serde() {
        let info = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new([0xab; 20]),
                public_key: [0xcd; 32],
                nonce: 99,
            },
            addr: "192.168.1.1:4433".parse().unwrap(),
            alt_addrs: vec!["[::1]:4433".parse().unwrap(), "10.0.0.1:4433".parse().unwrap()],
            capabilities: Capabilities::phase1_default(),
        };
        let bytes = rmp_serde::to_vec(&info).unwrap();
        let parsed: NodeInfo = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, info);
        assert_eq!(parsed.alt_addrs.len(), 2);
        assert!(parsed.alt_addrs[0].is_ipv6());
    }

    #[test]
    fn node_info_empty_alt_addrs_roundtrip() {
        // Empty alt_addrs serializes as empty array and deserializes correctly
        let info = NodeInfo {
            identity: NodeIdentity {
                node_id: NodeId::new([0xab; 20]),
                public_key: [0xcd; 32],
                nonce: 99,
            },
            addr: "192.168.1.1:4433".parse().unwrap(),
            alt_addrs: vec![],
            capabilities: Capabilities::phase1_default(),
        };
        let bytes = rmp_serde::to_vec(&info).unwrap();
        let parsed: NodeInfo = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, info);
        assert!(parsed.alt_addrs.is_empty());
    }
}
