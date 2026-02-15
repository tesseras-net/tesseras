//! Integration test: hole-punch flow via introducer.
//!
//! Three nodes on loopback using MemTransport:
//! - A (NATed initiator)
//! - B (NATed target)
//! - I (Introducer, public)
//!
//! Flow:
//! 1. A sends PunchIntro to I
//! 2. I forwards PunchRequest to B
//! 3. B replies PunchReady to A (via I for simplicity)
//! 4. A and B can now exchange messages directly

use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use tesseras_core::NodeId;
use tesseras_core::network::NodeIdentity;
use tesseras_dht::message::{self, Message};
use tesseras_net::punch::{sign_punch_intro, verify_punch_intro};
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
async fn hole_punch_full_flow() {
    // Set up simulated network with 3 nodes
    let net = SimNetwork::new();
    let node_a = net.create_transport(addr(5001), 16).await; // NATed initiator
    let node_b = net.create_transport(addr(5002), 16).await; // NATed target
    let node_i = net.create_transport(addr(5003), 16).await; // Introducer (public)

    let identity_a = make_identity(1);
    let identity_b = make_identity(2);
    let identity_i = make_identity(3);

    // A generates a signing key for the punch intro
    let signing_key_a = SigningKey::generate(&mut OsRng);
    let verifying_key_a = signing_key_a.verifying_key();

    let target_id = identity_b.node_id;
    let a_external_addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
    let timestamp = now_secs();

    // Step 1: A signs and sends PunchIntro to I
    let signature = sign_punch_intro(&target_id, &a_external_addr, timestamp, &signing_key_a);
    let punch_intro = Message::PunchIntro {
        sender: identity_a.clone(),
        target: target_id,
        external_addr: a_external_addr,
        timestamp,
        signature: signature.to_vec(),
    };
    send_msg(&node_a, &peer(5003), &punch_intro).await;

    // Step 2: I receives PunchIntro, verifies signature, forwards PunchRequest to B
    let (from, received) = recv_msg(&node_i).await;
    assert_eq!(from.addr, addr(5001));
    match &received {
        Message::PunchIntro {
            sender,
            target,
            external_addr,
            timestamp: ts,
            signature: sig,
        } => {
            assert_eq!(sender, &identity_a);
            assert_eq!(target, &target_id);
            assert_eq!(external_addr, &a_external_addr);

            // Verify the signature (I would use A's public key in practice)
            let sig_bytes: [u8; 64] = sig.as_slice().try_into().unwrap();
            verify_punch_intro(target, external_addr, *ts, &sig_bytes, &verifying_key_a).unwrap();

            // Forward as PunchRequest to B
            let punch_request = Message::PunchRequest {
                sender: identity_i.clone(),
                initiator: sender.clone(),
                initiator_addr: *external_addr,
                timestamp: *ts,
                signature: sig.clone(),
            };
            send_msg(&node_i, &peer(5002), &punch_request).await;
        }
        other => panic!("expected PunchIntro, got: {other:?}"),
    }

    // Step 3: B receives PunchRequest, verifies signature, sends PunchReady
    let (from, received) = recv_msg(&node_b).await;
    assert_eq!(from.addr, addr(5003)); // from introducer
    match &received {
        Message::PunchRequest {
            sender,
            initiator,
            initiator_addr,
            timestamp: ts,
            signature: sig,
        } => {
            assert_eq!(sender, &identity_i);
            assert_eq!(initiator, &identity_a);
            assert_eq!(initiator_addr, &a_external_addr);

            // B verifies the original signature from A
            let sig_bytes: [u8; 64] = sig.as_slice().try_into().unwrap();
            verify_punch_intro(
                &target_id,
                initiator_addr,
                *ts,
                &sig_bytes,
                &verifying_key_a,
            )
            .unwrap();

            // B sends PunchReady back to A (in practice, B would also send UDP
            // to A's external address to open the NAT pinhole)
            let b_external_addr: SocketAddr = "198.51.100.10:4433".parse().unwrap();
            let punch_ready = Message::PunchReady {
                sender: identity_b.clone(),
                external_addr: b_external_addr,
            };
            // Send PunchReady to A (via I in this test since MemTransport doesn't
            // simulate NAT; in production this goes directly)
            send_msg(&node_b, &peer(5001), &punch_ready).await;
        }
        other => panic!("expected PunchRequest, got: {other:?}"),
    }

    // Step 4: A receives PunchReady from B
    let (from, received) = recv_msg(&node_a).await;
    assert_eq!(from.addr, addr(5002)); // from B
    match &received {
        Message::PunchReady {
            sender,
            external_addr,
        } => {
            assert_eq!(sender, &identity_b);
            assert_eq!(
                external_addr,
                &"198.51.100.10:4433".parse::<SocketAddr>().unwrap()
            );
        }
        other => panic!("expected PunchReady, got: {other:?}"),
    }

    // Step 5: Verify A and B can now exchange directly
    send_msg(
        &node_a,
        &peer(5002),
        &Message::Ping {
            sender: identity_a.clone(),
        },
    )
    .await;
    let (from, msg) = recv_msg(&node_b).await;
    assert_eq!(from.addr, addr(5001));
    assert!(matches!(msg, Message::Ping { .. }));
}

#[tokio::test]
async fn punch_intro_with_bad_signature_rejected() {
    let net = SimNetwork::new();
    let node_a = net.create_transport(addr(6001), 16).await;
    let node_i = net.create_transport(addr(6002), 16).await;

    let identity_a = make_identity(10);
    let target_id = NodeId::new([20; 20]);
    let a_external_addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
    let timestamp = now_secs();

    // Send with garbage signature
    let punch_intro = Message::PunchIntro {
        sender: identity_a.clone(),
        target: target_id,
        external_addr: a_external_addr,
        timestamp,
        signature: vec![0xFF; 64],
    };
    send_msg(&node_a, &peer(6002), &punch_intro).await;

    // I receives it and tries to verify with a random key — should fail
    let (_from, received) = recv_msg(&node_i).await;
    match &received {
        Message::PunchIntro {
            target,
            external_addr,
            timestamp: ts,
            signature: sig,
            ..
        } => {
            let random_key = SigningKey::generate(&mut OsRng);
            let sig_bytes: [u8; 64] = sig.as_slice().try_into().unwrap();
            let result = verify_punch_intro(
                target,
                external_addr,
                *ts,
                &sig_bytes,
                &random_key.verifying_key(),
            );
            assert!(result.is_err(), "bad signature should not verify");
        }
        other => panic!("expected PunchIntro, got: {other:?}"),
    }
}
