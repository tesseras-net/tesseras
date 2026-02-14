use serde::{Deserialize, Serialize};

use tesseras_core::{Capabilities, ContentHash, NodeId, NodeIdentity, NodeInfo, TesseraPointer};

/// Kademlia protocol messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Ping {
        sender: NodeIdentity,
    },
    Pong {
        sender: NodeIdentity,
        capabilities: Capabilities,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindValueResult {
    Found(TesseraPointer),
    Nodes(Vec<NodeInfo>),
}

/// Encode a Message to MessagePack bytes.
pub fn encode(msg: &Message) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec(msg).map_err(|e| e.to_string())
}

/// Decode a Message from MessagePack bytes.
pub fn decode(data: &[u8]) -> Result<Message, String> {
    rmp_serde::from_slice(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
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
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, msg);
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
        if let (Message::Store { key: k1, .. }, Message::Store { key: k2, .. }) =
            (&msg, &decoded)
        {
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
}
