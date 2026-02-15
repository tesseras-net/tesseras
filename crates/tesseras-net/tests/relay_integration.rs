//! Integration test: relay flow with session migration.
//!
//! Three nodes on loopback using MemTransport:
//! - A (NATed peer)
//! - B (NATed peer)
//! - R (Relay node, public)
//!
//! Flow:
//! 1. A sends RelayRequest to R
//! 2. R creates session, sends RelayOffer to both A and B
//! 3. A sends token-prefixed packet to R → R forwards to B
//! 4. B replies through R → R forwards to A
//! 5. A migrates address (RelayMigrate)
//! 6. A sends RelayClose → session torn down

use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tesseras_core::NodeId;
use tesseras_core::network::NodeIdentity;
use tesseras_dht::message::{self, Message, RelayCloseReason};
use tesseras_net::punch::sign_relay_request;
use tesseras_net::relay::{ForwardResult, RelaySessionManager};
use tesseras_net::transport::PeerAddr;
use tesseras_net::{MemTransport, SimNetwork, Transport};

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn peer(port: u16) -> PeerAddr {
    PeerAddr {
        node_id: None,
        addr: addr(port),
    }
}

fn make_identity(seed: u8) -> NodeIdentity {
    NodeIdentity {
        node_id: NodeId::new([seed; 20]),
        public_key: [seed; 32],
        nonce: seed as u64,
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn send_msg(transport: &MemTransport, peer: &PeerAddr, msg: &Message) {
    let bytes = message::encode(msg).unwrap();
    transport.send(peer, &bytes).await.unwrap();
}

async fn recv_msg(transport: &MemTransport) -> (PeerAddr, Message) {
    let env = transport.recv().await.unwrap();
    let msg = message::decode(&env.payload).unwrap();
    (env.peer, msg)
}

#[tokio::test]
async fn relay_full_flow_with_migration_and_close() {
    let net = SimNetwork::new();
    let node_a = net.create_transport(addr(7001), 16).await; // NATed peer A
    let node_b = net.create_transport(addr(7002), 16).await; // NATed peer B
    let node_r = net.create_transport(addr(7003), 16).await; // Relay

    let identity_a = make_identity(1);
    let identity_b = make_identity(2);
    let identity_r = make_identity(3);

    let signing_key_a = SigningKey::generate(&mut OsRng);
    let timestamp = now_secs();

    // Step 1: A sends RelayRequest to R
    let signature = sign_relay_request(&identity_b.node_id, timestamp, &signing_key_a);
    let relay_request = Message::RelayRequest {
        sender: identity_a.clone(),
        target: identity_b.node_id,
        timestamp,
        signature: signature.to_vec(),
    };
    send_msg(&node_a, &peer(7003), &relay_request).await;

    // R receives RelayRequest
    let (from, received) = recv_msg(&node_r).await;
    assert_eq!(from.addr, addr(7001));
    assert!(matches!(received, Message::RelayRequest { .. }));

    // Step 2: R creates a relay session and sends RelayOffer to both peers
    let relay_mgr = RelaySessionManager::new(Duration::from_secs(60));
    let session_token = relay_mgr.create_session(
        addr(7001), // A
        addr(7002), // B
        identity_a.public_key,
        identity_b.public_key,
        true, // reciprocal
    );

    let relay_offer_a = Message::RelayOffer {
        sender: identity_r.clone(),
        relay_addr: addr(7003),
        session_token,
    };
    let relay_offer_b = Message::RelayOffer {
        sender: identity_r.clone(),
        relay_addr: addr(7003),
        session_token,
    };
    send_msg(&node_r, &peer(7001), &relay_offer_a).await;
    send_msg(&node_r, &peer(7002), &relay_offer_b).await;

    // A receives RelayOffer
    let (from, received_a) = recv_msg(&node_a).await;
    assert_eq!(from.addr, addr(7003));
    let token_a = match &received_a {
        Message::RelayOffer {
            session_token,
            relay_addr,
            ..
        } => {
            assert_eq!(*relay_addr, addr(7003));
            *session_token
        }
        other => panic!("expected RelayOffer, got: {other:?}"),
    };

    // B receives RelayOffer
    let (from, received_b) = recv_msg(&node_b).await;
    assert_eq!(from.addr, addr(7003));
    let token_b = match &received_b {
        Message::RelayOffer { session_token, .. } => *session_token,
        other => panic!("expected RelayOffer, got: {other:?}"),
    };
    assert_eq!(token_a, token_b);

    // Step 3: A sends token-prefixed data to R → R forwards to B
    let mut pkt_a_to_b = Vec::new();
    pkt_a_to_b.extend_from_slice(&token_a);
    pkt_a_to_b.extend_from_slice(b"hello from A");

    let result = relay_mgr.forward(&pkt_a_to_b, addr(7001));
    match result {
        ForwardResult::Forward {
            dest,
            payload_offset,
        } => {
            assert_eq!(dest, addr(7002));
            assert_eq!(&pkt_a_to_b[payload_offset..], b"hello from A");
            // Relay forwards the payload part to B
            node_r
                .send(&peer(7002), &pkt_a_to_b[payload_offset..])
                .await
                .unwrap();
        }
        other => panic!("expected Forward, got: {other:?}"),
    }

    // B receives forwarded payload
    let env = node_b.recv().await.unwrap();
    assert_eq!(env.payload, b"hello from A");

    // Step 4: B replies through R → R forwards to A
    let mut pkt_b_to_a = Vec::new();
    pkt_b_to_a.extend_from_slice(&token_b);
    pkt_b_to_a.extend_from_slice(b"hello from B");

    let result = relay_mgr.forward(&pkt_b_to_a, addr(7002));
    match result {
        ForwardResult::Forward {
            dest,
            payload_offset,
        } => {
            assert_eq!(dest, addr(7001));
            node_r
                .send(&peer(7001), &pkt_b_to_a[payload_offset..])
                .await
                .unwrap();
        }
        other => panic!("expected Forward, got: {other:?}"),
    }

    // A receives forwarded payload
    let env = node_a.recv().await.unwrap();
    assert_eq!(env.payload, b"hello from B");

    // Step 5: A migrates address (simulate network change)
    assert!(relay_mgr.migrate_peer(&token_a, addr(7001), addr(7004)));

    // Create new transport for A's new address
    let node_a_new = net.create_transport(addr(7004), 16).await;

    // A sends from new address
    let mut pkt_a_new = Vec::new();
    pkt_a_new.extend_from_slice(&token_a);
    pkt_a_new.extend_from_slice(b"migrated A");

    let result = relay_mgr.forward(&pkt_a_new, addr(7004));
    match result {
        ForwardResult::Forward {
            dest,
            payload_offset,
        } => {
            assert_eq!(dest, addr(7002));
            node_a_new
                .send(&peer(7002), &pkt_a_new[payload_offset..])
                .await
                .unwrap();
        }
        other => panic!("expected Forward after migration, got: {other:?}"),
    }

    let env = node_b.recv().await.unwrap();
    assert_eq!(env.payload, b"migrated A");

    // Old address should be rejected
    let result = relay_mgr.forward(&pkt_a_to_b, addr(7001));
    assert_eq!(result, ForwardResult::UnauthorizedSource);

    // Step 6: A sends RelayClose
    let relay_close = Message::RelayClose {
        session_token: token_a,
        reason: RelayCloseReason::PeerClosed,
    };
    send_msg(&node_a_new, &peer(7003), &relay_close).await;

    // R receives RelayClose and tears down session
    let (from, received) = recv_msg(&node_r).await;
    assert_eq!(from.addr, addr(7004)); // from migrated address
    match &received {
        Message::RelayClose {
            session_token,
            reason,
        } => {
            assert_eq!(session_token, &token_a);
            assert_eq!(reason, &RelayCloseReason::PeerClosed);

            // R removes the session
            let removed = relay_mgr.remove_session(session_token);
            assert!(removed.is_some());
        }
        other => panic!("expected RelayClose, got: {other:?}"),
    }

    // Verify session is torn down
    assert_eq!(relay_mgr.active_sessions(), 0);

    // Further forwarding should fail
    let result = relay_mgr.forward(&pkt_a_new, addr(7004));
    assert_eq!(result, ForwardResult::UnknownSession);
}
