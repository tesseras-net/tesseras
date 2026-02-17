//! Property-based tests for serialization roundtrips.
//!
//! Uses proptest to generate arbitrary domain types and verify that
//! serialization → deserialization produces the original value.

use proptest::prelude::*;
use tesseras::dht::{DhtMessage, PeerInfo, SignedEnvelope};
use tesseras::rpc::{RpcRequest, RpcResponse};
use tesseras::types::{ContentHash, MediaType, Memory, NodeId, Tessera, Visibility};

// --- Arbitrary strategies ---

fn arb_content_hash() -> impl Strategy<Value = ContentHash> {
    prop::array::uniform32(any::<u8>()).prop_map(ContentHash::new)
}

fn arb_node_id() -> impl Strategy<Value = NodeId> {
    prop::array::uniform32(any::<u8>()).prop_map(NodeId::new)
}

fn arb_socket_addr() -> impl Strategy<Value = std::net::SocketAddr> {
    (any::<[u8; 4]>(), 1024u16..=65535u16)
        .prop_map(|(ip, port)| std::net::SocketAddr::from((ip, port)))
}

fn arb_peer_info() -> impl Strategy<Value = PeerInfo> {
    (arb_node_id(), arb_socket_addr()).prop_map(|(node_id, addr)| PeerInfo { node_id, addr })
}

fn arb_visibility() -> impl Strategy<Value = Visibility> {
    prop_oneof![
        Just(Visibility::Public),
        Just(Visibility::Private),
        "[a-z]{1,10}".prop_map(|name| Visibility::Circle { name }),
    ]
}

fn arb_media_type() -> impl Strategy<Value = MediaType> {
    prop_oneof![
        Just(MediaType::Image),
        Just(MediaType::Audio),
        Just(MediaType::Video),
        Just(MediaType::Text),
    ]
}

fn arb_memory() -> impl Strategy<Value = Memory> {
    (
        "[a-z]{1,20}\\.[a-z]{2,4}",
        arb_media_type(),
        0u64..1_000_000,
        arb_content_hash(),
    )
        .prop_map(|(filename, media_type, size, blob_hash)| Memory {
            filename,
            media_type,
            size,
            blob_hash,
        })
}

fn arb_tessera() -> impl Strategy<Value = Tessera> {
    (
        arb_content_hash(),
        prop::collection::vec(any::<u8>(), 32..=32),
        prop::collection::vec(any::<u8>(), 64..=64),
        prop::option::of("[a-z ]{1,30}"),
        arb_visibility(),
        prop::collection::vec(arb_memory(), 0..5),
    )
        .prop_map(
            |(hash, author, signature, name, visibility, memories)| Tessera {
                hash,
                author,
                signature,
                created_at: chrono::Utc::now(),
                name,
                visibility,
                memories,
            },
        )
}

fn arb_dht_message() -> impl Strategy<Value = DhtMessage> {
    prop_oneof![
        arb_node_id().prop_map(|sender| DhtMessage::Ping { sender }),
        arb_node_id().prop_map(|sender| DhtMessage::Pong { sender }),
        (arb_node_id(), arb_node_id())
            .prop_map(|(sender, target)| DhtMessage::FindNode { sender, target }),
        (arb_node_id(), prop::collection::vec(arb_peer_info(), 0..5))
            .prop_map(|(sender, closest)| DhtMessage::FindNodeResponse { sender, closest }),
        (arb_node_id(), arb_content_hash())
            .prop_map(|(sender, key)| DhtMessage::FindValue { sender, key }),
        (arb_node_id(), arb_content_hash())
            .prop_map(|(sender, hash)| DhtMessage::FetchBlob { sender, hash }),
        (arb_node_id(), any::<bool>(), any::<u64>()).prop_map(|(sender, found, size)| {
            DhtMessage::FetchBlobResponse {
                sender,
                found,
                size,
            }
        }),
        (arb_node_id(), arb_content_hash(), arb_peer_info()).prop_map(|(sender, key, provider)| {
            DhtMessage::Store {
                sender,
                key,
                provider,
            }
        }),
        (arb_node_id(), any::<bool>())
            .prop_map(|(sender, success)| DhtMessage::StoreResponse { sender, success }),
        (arb_node_id(), arb_content_hash())
            .prop_map(|(sender, key)| DhtMessage::Retract { sender, key }),
        (arb_node_id(), any::<bool>())
            .prop_map(|(sender, success)| DhtMessage::RetractResponse { sender, success }),
        (arb_node_id(), arb_content_hash())
            .prop_map(|(sender, hash)| DhtMessage::FetchTessera { sender, hash }),
        (
            arb_node_id(),
            arb_node_id(),
            prop::collection::vec(any::<u8>(), 0..100)
        )
            .prop_map(|(sender, target, payload)| DhtMessage::RelayRequest {
                sender,
                target,
                payload,
            }),
    ]
}

// --- Roundtrip tests ---

proptest! {
    #[test]
    fn content_hash_display_parse_roundtrip(bytes in prop::array::uniform32(any::<u8>())) {
        let hash = ContentHash::new(bytes);
        let s = hash.to_string();
        let parsed: ContentHash = s.parse().unwrap();
        prop_assert_eq!(hash, parsed);
    }

    #[test]
    fn node_id_display_parse_roundtrip(bytes in prop::array::uniform32(any::<u8>())) {
        let id = NodeId::new(bytes);
        let s = id.to_string();
        let parsed: NodeId = s.parse().unwrap();
        prop_assert_eq!(id, parsed);
    }

    #[test]
    fn visibility_display_parse_roundtrip(vis in arb_visibility()) {
        let s = vis.to_string();
        let parsed: Visibility = s.parse().unwrap();
        prop_assert_eq!(vis, parsed);
    }

    #[test]
    fn dht_message_msgpack_roundtrip(msg in arb_dht_message()) {
        let bytes = msg.to_bytes();
        let decoded = DhtMessage::from_bytes(&bytes).unwrap();
        // Re-serialize to verify structural equality
        let re_encoded = decoded.to_bytes();
        prop_assert_eq!(bytes, re_encoded);
    }

    #[test]
    fn tessera_json_roundtrip(tessera in arb_tessera()) {
        let json = serde_json::to_string(&tessera).unwrap();
        let decoded: Tessera = serde_json::from_str(&json).unwrap();
        // Verify key fields
        prop_assert_eq!(tessera.hash, decoded.hash);
        prop_assert_eq!(tessera.author, decoded.author);
        prop_assert_eq!(tessera.name, decoded.name);
        prop_assert_eq!(tessera.visibility, decoded.visibility);
        prop_assert_eq!(tessera.memories.len(), decoded.memories.len());
    }

    #[test]
    fn tessera_msgpack_roundtrip(tessera in arb_tessera()) {
        let bytes = rmp_serde::to_vec(&tessera).unwrap();
        let decoded: Tessera = rmp_serde::from_slice(&bytes).unwrap();
        prop_assert_eq!(tessera.hash, decoded.hash);
        prop_assert_eq!(tessera.author, decoded.author);
        prop_assert_eq!(tessera.name, decoded.name);
        prop_assert_eq!(tessera.memories.len(), decoded.memories.len());
    }

    #[test]
    fn rpc_request_msgpack_roundtrip(
        variant in prop_oneof![
            Just(RpcRequest::Ping),
            Just(RpcRequest::ListTesseras),
            Just(RpcRequest::NodeStatus),
            Just(RpcRequest::CheckFragments),
            arb_content_hash().prop_map(|hash| RpcRequest::GetTessera { hash }),
            arb_content_hash().prop_map(|hash| RpcRequest::RemoveTessera { hash }),
            arb_content_hash().prop_map(|hash| RpcRequest::FetchTesseraFromNetwork { hash }),
            arb_content_hash().prop_map(|hash| RpcRequest::AnnounceTessera { hash }),
            arb_content_hash().prop_map(|hash| RpcRequest::DistributeFragments { hash }),
        ]
    ) {
        let bytes = rmp_serde::to_vec(&variant).unwrap();
        let decoded: RpcRequest = rmp_serde::from_slice(&bytes).unwrap();
        let re_encoded = rmp_serde::to_vec(&decoded).unwrap();
        prop_assert_eq!(bytes, re_encoded);
    }

    #[test]
    fn rpc_response_msgpack_roundtrip(
        variant in prop_oneof![
            Just(RpcResponse::Ok),
            ".*".prop_map(RpcResponse::Error),
            (any::<usize>(), ".*").prop_map(|(pc, addr)| RpcResponse::Pong {
                node_id: "abc".into(),
                peer_count: pc % 100,
                listen_addr: addr,
            }),
            (any::<usize>(), any::<usize>(), ".*").prop_map(|(pc, tc, addr)| RpcResponse::Status {
                node_id: "def".into(),
                peer_count: pc % 100,
                tessera_count: tc % 100,
                listen_addr: addr,
                total_storage_bytes: 0,
                foreign_storage_bytes: 0,
            }),
        ]
    ) {
        let bytes = rmp_serde::to_vec(&variant).unwrap();
        let decoded: RpcResponse = rmp_serde::from_slice(&bytes).unwrap();
        let re_encoded = rmp_serde::to_vec(&decoded).unwrap();
        prop_assert_eq!(bytes, re_encoded);
    }

    #[test]
    fn signed_envelope_msgpack_roundtrip(
        payload in prop::collection::vec(any::<u8>(), 0..200),
        public_key in prop::collection::vec(any::<u8>(), 32..=32),
        signature in prop::collection::vec(any::<u8>(), 64..=64),
    ) {
        let envelope = SignedEnvelope { payload, public_key, signature };
        let bytes = rmp_serde::to_vec(&envelope).unwrap();
        let decoded: SignedEnvelope = rmp_serde::from_slice(&bytes).unwrap();
        prop_assert_eq!(envelope.payload, decoded.payload);
        prop_assert_eq!(envelope.public_key, decoded.public_key);
        prop_assert_eq!(envelope.signature, decoded.signature);
    }

    #[test]
    fn xor_distance_symmetric(a_bytes in prop::array::uniform32(any::<u8>()), b_bytes in prop::array::uniform32(any::<u8>())) {
        let a = NodeId::new(a_bytes);
        let b = NodeId::new(b_bytes);
        prop_assert_eq!(a.distance(&b), b.distance(&a));
    }

    #[test]
    fn xor_distance_self_is_zero(bytes in prop::array::uniform32(any::<u8>())) {
        let a = NodeId::new(bytes);
        prop_assert_eq!(a.distance(&a), [0u8; 32]);
    }
}
