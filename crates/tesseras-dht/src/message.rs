use serde::{Deserialize, Serialize};

use tesseras_core::network::NatType;
use tesseras_core::replication::{Attestation, FragmentEnvelope, ReplicateAck};
use tesseras_core::search::{SearchFilters, SearchHit};
use tesseras_core::{Capabilities, ContentHash, NodeId, NodeIdentity, NodeInfo, TesseraPointer};

/// Why a relay session was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayCloseReason {
    /// Peer explicitly closed.
    PeerClosed,
    /// No packets for idle timeout period.
    IdleTimeout,
    /// Rate limit exceeded.
    RateLimitExceeded,
    /// Relay node shutting down.
    RelayShutdown,
}

/// Kademlia protocol messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Ping {
        sender: NodeIdentity,
    },
    Pong {
        sender: NodeIdentity,
        capabilities: Capabilities,
        /// Additional listen addresses (e.g. IPv6). Empty for single-address nodes.
        #[serde(default)]
        listen_addrs: Vec<std::net::SocketAddr>,
        /// Detected NAT type of the sender.
        #[serde(default)]
        nat_type: Option<NatType>,
        /// Available relay slots (only if RELAY capability is set).
        #[serde(default)]
        relay_slots_available: Option<u16>,
        /// Current relay bandwidth usage in KB/s.
        #[serde(default)]
        relay_bandwidth_used_kbps: Option<u32>,
    },

    FindNode {
        target: NodeId,
    },
    FindNodeResponse {
        nodes: Vec<NodeInfo>,
    },

    FindValue {
        key: ContentHash,
    },
    FindValueResponse {
        result: FindValueResult,
    },

    Store {
        key: ContentHash,
        pointer: TesseraPointer,
    },
    StoreResponse {
        accepted: bool,
    },

    Replicate {
        envelope: FragmentEnvelope,
    },
    ReplicateAck {
        ack: ReplicateAck,
    },
    AttestRequest {
        tessera_hash: ContentHash,
    },
    AttestResponse {
        attestation: Attestation,
    },

    // --- NAT Traversal (Phase 4) ---
    /// Request hole-punch introduction. Signed to prevent reflection attacks.
    PunchIntro {
        sender: NodeIdentity,
        target: NodeId,
        /// Initiator's external address (from STUN).
        external_addr: std::net::SocketAddr,
        /// Prevents replay (seconds since UNIX epoch).
        timestamp: u64,
        /// Ed25519 signature over (target || external_addr || timestamp).
        signature: Vec<u8>,
    },

    /// Introducer forwards punch request to target.
    /// Carries initiator's original signature for direct verification.
    PunchRequest {
        sender: NodeIdentity,
        initiator: NodeIdentity,
        initiator_addr: std::net::SocketAddr,
        timestamp: u64,
        /// Original signature from PunchIntro.
        signature: Vec<u8>,
    },

    /// Target confirms readiness for hole-punch.
    PunchReady {
        sender: NodeIdentity,
        /// Target's external address (from STUN).
        external_addr: std::net::SocketAddr,
    },

    /// Request a relay session through a public-IP node.
    RelayRequest {
        sender: NodeIdentity,
        target: NodeId,
        timestamp: u64,
        /// Ed25519 signature over (target || timestamp).
        signature: Vec<u8>,
    },

    /// Relay session established. Sent to both peers.
    RelayOffer {
        sender: NodeIdentity,
        /// The relay's UDP address to send QUIC packets to.
        relay_addr: std::net::SocketAddr,
        /// Opaque token identifying this session at the relay.
        session_token: [u8; 16],
    },

    /// Relay session closed.
    RelayClose {
        session_token: [u8; 16],
        reason: RelayCloseReason,
    },

    /// Request to migrate relay session to new source address (after network change).
    RelayMigrate {
        session_token: [u8; 16],
        timestamp: u64,
        /// Ed25519 signature over (session_token || timestamp).
        signature: Vec<u8>,
    },

    // --- Tombstone retraction ---
    /// Creator requests deletion of a tessera from the network.
    Retract {
        tombstone: tesseras_core::Tombstone,
    },
    /// Acknowledgement of a retraction request.
    RetractAck {
        hash: ContentHash,
        accepted: bool,
    },

    // --- Institutional Search (Phase 5) ---
    /// Full-text search request forwarded to institutional nodes with SEARCH_INDEX capability.
    Search {
        query: String,
        filters: SearchFilters,
        page: u32,
    },
    /// Search results returned by an institutional node.
    SearchResult {
        hits: Vec<SearchHit>,
        total: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindValueResult {
    Found(TesseraPointer),
    Nodes(Vec<NodeInfo>),
}

/// Encode a Message to MessagePack bytes (named fields for forward-compatibility).
pub fn encode(msg: &Message) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(msg).map_err(|e| e.to_string())
}

/// Decode a Message from MessagePack bytes.
pub fn decode(data: &[u8]) -> Result<Message, String> {
    rmp_serde::from_slice(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tesseras_core::replication::{AttestationEntry, FragmentId, FragmentPlan};
    use tesseras_core::{HolderInfo, Visibility};

    fn test_identity() -> NodeIdentity {
        NodeIdentity {
            node_id: NodeId::new([0x01; 20]),
            public_key: [0x02; 32],
            nonce: 42,
        }
    }

    fn test_pointer() -> TesseraPointer {
        TesseraPointer {
            tessera_hash: ContentHash::new([0xaa; 32]),
            size_bytes: 1_000_000,
            holders: vec![HolderInfo {
                node_id: NodeId::new([0x03; 20]),
                addr: "10.0.0.1:4433".parse().unwrap(),
                alt_addrs: vec![],
                last_seen: chrono::Utc::now(),
                fragments: vec![0, 1, 2],
            }],
            visibility: Visibility::Public,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn ping_roundtrip() {
        let msg = Message::Ping {
            sender: test_identity(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn pong_roundtrip() {
        let msg = Message::Pong {
            sender: test_identity(),
            capabilities: Capabilities::phase1_default(),
            listen_addrs: vec![],
            nat_type: None,
            relay_slots_available: None,
            relay_bandwidth_used_kbps: None,
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn pong_with_listen_addrs_roundtrip() {
        let msg = Message::Pong {
            sender: test_identity(),
            capabilities: Capabilities::phase1_default(),
            listen_addrs: vec![
                "[::1]:4433".parse().unwrap(),
                "10.0.0.1:4433".parse().unwrap(),
            ],
            nat_type: None,
            relay_slots_available: None,
            relay_bandwidth_used_kbps: None,
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn pong_backward_compat() {
        // Pong without listen_addrs (old format) should deserialize fine
        let msg_old = Message::Pong {
            sender: test_identity(),
            capabilities: Capabilities::phase1_default(),
            listen_addrs: vec![],
            nat_type: None,
            relay_slots_available: None,
            relay_bandwidth_used_kbps: None,
        };
        let bytes = encode(&msg_old).unwrap();
        let decoded = decode(&bytes).unwrap();
        if let Message::Pong { listen_addrs, .. } = decoded {
            assert!(listen_addrs.is_empty());
        } else {
            panic!("expected Pong");
        }
    }

    #[test]
    fn find_node_roundtrip() {
        let msg = Message::FindNode {
            target: NodeId::new([0xff; 20]),
        };
        let bytes = encode(&msg).unwrap();
        assert_eq!(decode(&bytes).unwrap(), msg);
    }

    #[test]
    fn find_node_response_roundtrip() {
        let msg = Message::FindNodeResponse {
            nodes: vec![NodeInfo {
                identity: test_identity(),
                addr: "192.168.1.1:4433".parse::<SocketAddr>().unwrap(),
                alt_addrs: vec![],
                capabilities: Capabilities::phase1_default(),
            }],
        };
        let bytes = encode(&msg).unwrap();
        assert_eq!(decode(&bytes).unwrap(), msg);
    }

    #[test]
    fn store_roundtrip() {
        let msg = Message::Store {
            key: ContentHash::new([0xbb; 32]),
            pointer: test_pointer(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        if let (Message::Store { key: k1, .. }, Message::Store { key: k2, .. }) = (&msg, &decoded) {
            assert_eq!(k1, k2);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn find_value_found_roundtrip() {
        let msg = Message::FindValueResponse {
            result: FindValueResult::Found(test_pointer()),
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(
            decoded,
            Message::FindValueResponse {
                result: FindValueResult::Found(_)
            }
        ));
    }

    #[test]
    fn replicate_roundtrip() {
        let envelope = FragmentEnvelope {
            id: FragmentId::new(
                ContentHash::new([0x01; 32]),
                0,
                16,
                ContentHash::new([0xcc; 32]),
            ),
            plan: FragmentPlan::new(ContentHash::new([0x01; 32]), 100_000_000).unwrap(),
            original_tessera_size: 100_000_000,
            fragment_size: 100_000_000 / 16,
            data: vec![0xaa; 64],
        };
        let msg = Message::Replicate { envelope };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(decoded, Message::Replicate { .. }));
    }

    #[test]
    fn attest_request_roundtrip() {
        let msg = Message::AttestRequest {
            tessera_hash: ContentHash::new([0x01; 32]),
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(decoded, Message::AttestRequest { .. }));
    }

    #[test]
    fn attest_response_roundtrip() {
        let msg = Message::AttestResponse {
            attestation: Attestation {
                tessera_hash: ContentHash::new([0x01; 32]),
                entries: vec![AttestationEntry {
                    fragment_index: 0,
                    checksum: ContentHash::new([0xcc; 32]),
                }],
                timestamp: chrono::Utc::now(),
                signature: vec![0xde, 0xad],
            },
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(decoded, Message::AttestResponse { .. }));
    }

    #[test]
    fn replicate_ack_roundtrip() {
        let msg = Message::ReplicateAck {
            ack: ReplicateAck {
                accepted: true,
                fragments_held: vec![0, 1, 2],
            },
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(decoded, Message::ReplicateAck { .. }));
    }

    #[test]
    fn find_value_nodes_roundtrip() {
        let msg = Message::FindValueResponse {
            result: FindValueResult::Nodes(vec![]),
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert!(matches!(
            decoded,
            Message::FindValueResponse {
                result: FindValueResult::Nodes(_)
            }
        ));
    }

    // --- NAT Traversal message round-trips ---

    fn roundtrip(msg: &Message) -> Message {
        let bytes = encode(msg).unwrap();
        decode(&bytes).unwrap()
    }

    #[test]
    fn punch_intro_roundtrip() {
        let msg = Message::PunchIntro {
            sender: test_identity(),
            target: NodeId::new([3u8; 20]),
            external_addr: "203.0.113.5:4433".parse().unwrap(),
            timestamp: 1234567890,
            signature: vec![0xAA; 64],
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::PunchIntro {
                target,
                timestamp,
                signature,
                ..
            } => {
                assert_eq!(target, NodeId::new([3u8; 20]));
                assert_eq!(timestamp, 1234567890);
                assert_eq!(signature, vec![0xAA; 64]);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn punch_request_roundtrip() {
        let msg = Message::PunchRequest {
            sender: test_identity(),
            initiator: test_identity(),
            initiator_addr: "203.0.113.5:4433".parse().unwrap(),
            timestamp: 1234567890,
            signature: vec![0xBB; 64],
        };
        assert!(matches!(roundtrip(&msg), Message::PunchRequest { .. }));
    }

    #[test]
    fn punch_ready_roundtrip() {
        let msg = Message::PunchReady {
            sender: test_identity(),
            external_addr: "203.0.113.5:4433".parse().unwrap(),
        };
        assert!(matches!(roundtrip(&msg), Message::PunchReady { .. }));
    }

    #[test]
    fn relay_request_roundtrip() {
        let msg = Message::RelayRequest {
            sender: test_identity(),
            target: NodeId::new([3u8; 20]),
            timestamp: 999,
            signature: vec![0xCC; 64],
        };
        assert!(matches!(roundtrip(&msg), Message::RelayRequest { .. }));
    }

    #[test]
    fn relay_offer_roundtrip() {
        let msg = Message::RelayOffer {
            sender: test_identity(),
            relay_addr: "198.51.100.1:5000".parse().unwrap(),
            session_token: [0xDD; 16],
        };
        assert!(matches!(roundtrip(&msg), Message::RelayOffer { .. }));
    }

    #[test]
    fn relay_close_roundtrip() {
        let msg = Message::RelayClose {
            session_token: [0xEE; 16],
            reason: RelayCloseReason::IdleTimeout,
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::RelayClose { reason, .. } => {
                assert_eq!(reason, RelayCloseReason::IdleTimeout);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn relay_migrate_roundtrip() {
        let msg = Message::RelayMigrate {
            session_token: [0xFF; 16],
            timestamp: 42,
            signature: vec![0x11; 64],
        };
        assert!(matches!(roundtrip(&msg), Message::RelayMigrate { .. }));
    }

    #[test]
    fn pong_with_nat_metadata_roundtrip() {
        let msg = Message::Pong {
            sender: test_identity(),
            capabilities: Capabilities::phase2_default(),
            listen_addrs: vec!["[::1]:4433".parse().unwrap()],
            nat_type: Some(NatType::Cone),
            relay_slots_available: Some(10),
            relay_bandwidth_used_kbps: Some(128),
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::Pong {
                nat_type,
                relay_slots_available,
                relay_bandwidth_used_kbps,
                ..
            } => {
                assert_eq!(nat_type, Some(NatType::Cone));
                assert_eq!(relay_slots_available, Some(10));
                assert_eq!(relay_bandwidth_used_kbps, Some(128));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn pong_backward_compatible_no_nat_fields() {
        let old_msg = Message::Pong {
            sender: test_identity(),
            capabilities: Capabilities::phase1_default(),
            listen_addrs: vec![],
            nat_type: None,
            relay_slots_available: None,
            relay_bandwidth_used_kbps: None,
        };
        let decoded = roundtrip(&old_msg);
        match decoded {
            Message::Pong {
                nat_type,
                relay_slots_available,
                ..
            } => {
                assert_eq!(nat_type, None);
                assert_eq!(relay_slots_available, None);
            }
            _ => panic!("wrong variant"),
        }
    }

    // --- Tombstone retraction message round-trips ---

    #[test]
    fn retract_roundtrip() {
        let msg = Message::Retract {
            tombstone: tesseras_core::Tombstone {
                hash: ContentHash::new([0xaa; 32]),
                retracted_at: chrono::Utc::now(),
                creator_pubkey: "ed25519:abc123".to_string(),
                ed25519_signature: vec![0xDE; 64],
                mldsa_signature: vec![0xAD; 128],
            },
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::Retract { tombstone } => {
                assert_eq!(tombstone.hash, ContentHash::new([0xaa; 32]));
                assert_eq!(tombstone.ed25519_signature.len(), 64);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn retract_ack_roundtrip() {
        let msg = Message::RetractAck {
            hash: ContentHash::new([0xbb; 32]),
            accepted: true,
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::RetractAck { hash, accepted } => {
                assert_eq!(hash, ContentHash::new([0xbb; 32]));
                assert!(accepted);
            }
            _ => panic!("wrong variant"),
        }
    }

    // --- Institutional Search message round-trips ---

    #[test]
    fn search_message_serde_roundtrip() {
        use tesseras_core::enums::MemoryType;
        use tesseras_core::search::SearchFilters;

        let msg = Message::Search {
            query: "memórias de São Paulo".to_string(),
            filters: SearchFilters {
                memory_type: Some(MemoryType::Moment),
                language: Some("pt-BR".to_string()),
                ..Default::default()
            },
            page: 0,
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::Search {
                query,
                filters,
                page,
            } => {
                assert_eq!(query, "memórias de São Paulo");
                assert_eq!(filters.language.as_deref(), Some("pt-BR"));
                assert_eq!(page, 0);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn search_result_message_serde_roundtrip() {
        use tesseras_core::enums::MemoryType;
        use tesseras_core::search::{MetadataExcerpt, SearchHit};

        let msg = Message::SearchResult {
            hits: vec![SearchHit {
                hash: ContentHash::new([0xaa; 32]),
                metadata: MetadataExcerpt {
                    title: Some("Dia em SP".to_string()),
                    description: Some("Um dia qualquer".to_string()),
                    memory_type: Some(MemoryType::Daily),
                    created_at: Some(chrono::Utc::now()),
                    visibility: Visibility::Public,
                    language: Some("pt-BR".to_string()),
                    tags: vec!["cotidiano".into()],
                },
            }],
            total: 42,
        };
        let decoded = roundtrip(&msg);
        match decoded {
            Message::SearchResult { hits, total } => {
                assert_eq!(total, 42);
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].metadata.title.as_deref(), Some("Dia em SP"));
            }
            _ => panic!("wrong variant"),
        }
    }
}
