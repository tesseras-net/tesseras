use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::{Mutex, oneshot};

use tesseras_core::*;
use tesseras_net::codec::{self as wire_codec, WireBody, WireMessage};
use tesseras_net::{Envelope, PeerAddr, Transport};

use crate::config::DhtConfig;
use crate::distance;
use crate::error::DhtError;
use crate::message::{self, FindValueResult, Message};
use crate::pow;
use crate::routing::RoutingTable;
use crate::store::{PointerStore, StoreConfig};

pub struct DhtEngine {
    identity: NodeIdentity,
    transport: Box<dyn Transport>,
    config: DhtConfig,
    routing: Mutex<RoutingTable>,
    store: Mutex<PointerStore>,
    request_counter: Mutex<u64>,
    pending: Mutex<HashMap<u64, oneshot::Sender<(WireMessage, PeerAddr)>>>,
}

impl DhtEngine {
    pub fn new(
        identity: NodeIdentity,
        transport: Box<dyn Transport>,
        config: DhtConfig,
    ) -> Arc<Self> {
        let store_config = StoreConfig {
            max_entries: config.max_stored_pointers,
            ttl: config.pointer_ttl,
        };
        Arc::new(Self {
            routing: Mutex::new(RoutingTable::new(identity.node_id)),
            store: Mutex::new(PointerStore::new(identity.node_id, store_config)),
            identity,
            transport,
            config,
            request_counter: Mutex::new(0),
            pending: Mutex::new(HashMap::new()),
        })
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }

    async fn next_request_id(&self) -> u64 {
        let mut counter = self.request_counter.lock().await;
        *counter += 1;
        *counter
    }

    /// Record a seen peer in the routing table.
    async fn record_peer(
        &self,
        peer_addr: SocketAddr,
        identity: &NodeIdentity,
        caps: Capabilities,
    ) {
        let info = NodeInfo {
            identity: identity.clone(),
            addr: peer_addr,
            capabilities: caps,
        };
        self.routing.lock().await.update(info);
    }

    /// Handle an incoming envelope: decode wire message, dispatch, and send response.
    pub async fn handle_envelope(&self, envelope: Envelope) {
        let (wire_msg, _) = match wire_codec::decode(&envelope.payload) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("failed to decode wire message: {e}");
                return;
            }
        };

        let body_bytes = match &wire_msg.body {
            WireBody::Request(b) => b,
            WireBody::Response(_) | WireBody::Error { .. } => {
                // Route to pending RPC caller
                let mut pending = self.pending.lock().await;
                if let Some(tx) = pending.remove(&wire_msg.request_id) {
                    let _ = tx.send((wire_msg, envelope.peer));
                }
                return;
            }
        };

        let msg = match message::decode(body_bytes) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("failed to decode DHT message: {e}");
                return;
            }
        };

        let response = self.handle_message(&msg, &envelope.peer).await;

        if let Some(resp_msg) = response {
            let resp_bytes = match message::encode(&resp_msg) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("failed to encode response: {e}");
                    return;
                }
            };
            let wire_resp = WireMessage {
                version: 1,
                request_id: wire_msg.request_id,
                body: WireBody::Response(resp_bytes),
            };
            let wire_bytes = match wire_codec::encode(&wire_resp) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("failed to encode wire response: {e}");
                    return;
                }
            };
            if let Some(resp_tx) = envelope.response_tx {
                // Reply on the same bidirectional stream (QUIC transport).
                let _ = resp_tx.send(wire_bytes);
            } else {
                // Fallback for transports without a response channel (e.g. MemTransport).
                if let Err(e) = self.transport.send(&envelope.peer, &wire_bytes).await {
                    tracing::warn!("failed to send response: {e}");
                }
            }
        }
    }

    /// Process a DHT message and return an optional response.
    async fn handle_message(&self, msg: &Message, peer: &PeerAddr) -> Option<Message> {
        match msg {
            Message::Ping { sender } => {
                if pow::verify_pow(sender) {
                    self.record_peer(peer.addr, sender, Capabilities::phase1_default())
                        .await;
                }
                Some(Message::Pong {
                    sender: self.identity.clone(),
                    capabilities: Capabilities::phase1_default(),
                })
            }
            Message::Pong {
                sender,
                capabilities,
            } => {
                if pow::verify_pow(sender) {
                    self.record_peer(peer.addr, sender, *capabilities).await;
                }
                None
            }
            Message::FindNode { target } => {
                let nodes = self.routing.lock().await.closest(target, self.config.k);
                Some(Message::FindNodeResponse { nodes })
            }
            Message::FindNodeResponse { nodes } => {
                let mut rt = self.routing.lock().await;
                for node in nodes {
                    if pow::verify_pow(&node.identity) {
                        rt.update(node.clone());
                    }
                }
                None
            }
            Message::FindValue { key } => {
                let store = self.store.lock().await;
                if let Some(ptr) = store.get(key) {
                    Some(Message::FindValueResponse {
                        result: FindValueResult::Found(ptr.clone()),
                    })
                } else {
                    drop(store);
                    // Truncate 32-byte ContentHash to 20-byte NodeId for closest lookup
                    let mut target_bytes = [0u8; 20];
                    target_bytes.copy_from_slice(&key.as_bytes()[..20]);
                    let target = NodeId::new(target_bytes);
                    let nodes = self.routing.lock().await.closest(&target, self.config.k);
                    Some(Message::FindValueResponse {
                        result: FindValueResult::Nodes(nodes),
                    })
                }
            }
            Message::FindValueResponse { .. } => None,
            Message::Store { pointer, .. } => {
                let accepted = self.store.lock().await.store(pointer.clone());
                Some(Message::StoreResponse { accepted })
            }
            Message::StoreResponse { .. } => None,
            Message::Replicate { .. } => {
                tracing::debug!("received REPLICATE (handler not yet wired)");
                None
            }
            Message::ReplicateAck { .. } => None,
            Message::AttestRequest { .. } => {
                tracing::debug!("received ATTEST_REQUEST (handler not yet wired)");
                None
            }
            Message::AttestResponse { .. } => None,
        }
    }

    /// Send an RPC and wait for the response. Returns None on timeout.
    async fn rpc(&self, peer: &PeerAddr, msg: &Message) -> Result<Option<Message>, DhtError> {
        let req_bytes = message::encode(msg).map_err(DhtError::Codec)?;
        let request_id = self.next_request_id().await;
        let wire_msg = WireMessage {
            version: 1,
            request_id,
            body: WireBody::Request(req_bytes),
        };
        let wire_bytes = wire_codec::encode(&wire_msg).map_err(DhtError::Transport)?;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, tx);

        self.transport.send(peer, &wire_bytes).await?;

        let result = tokio::time::timeout(self.config.rpc_timeout, rx).await;

        // Clean up on timeout (sender may already be removed if response arrived)
        self.pending.lock().await.remove(&request_id);

        match result {
            Ok(Ok((wire_resp, _peer))) => {
                if let WireBody::Response(resp_bytes) = wire_resp.body {
                    let resp = message::decode(&resp_bytes).map_err(DhtError::Codec)?;
                    Ok(Some(resp))
                } else if let WireBody::Error { reason, .. } = wire_resp.body {
                    Err(DhtError::LookupFailed(reason))
                } else {
                    Ok(None)
                }
            }
            Ok(Err(_)) => Ok(None), // sender dropped
            Err(_) => Ok(None),     // timeout
        }
    }

    /// Ping a peer and return whether it responded.
    pub async fn ping(&self, addr: SocketAddr) -> bool {
        let peer = PeerAddr {
            node_id: None,
            addr,
        };
        let msg = Message::Ping {
            sender: self.identity.clone(),
        };
        match self.rpc(&peer, &msg).await {
            Ok(Some(Message::Pong {
                sender,
                capabilities,
            })) => {
                if pow::verify_pow(&sender) {
                    self.record_peer(addr, &sender, capabilities).await;
                }
                true
            }
            _ => false,
        }
    }

    /// Iterative node lookup: find the k closest nodes to a target.
    pub async fn find_closest_nodes(&self, target: &NodeId) -> Vec<NodeInfo> {
        let k = self.config.k;
        let alpha = self.config.alpha;

        // Start with our closest known nodes
        let mut closest: Vec<NodeInfo> = self.routing.lock().await.closest(target, k);
        let mut queried: HashSet<NodeId> = HashSet::new();

        loop {
            // Pick alpha unqueried nodes from closest
            let to_query: Vec<NodeInfo> = closest
                .iter()
                .filter(|n| !queried.contains(&n.identity.node_id))
                .take(alpha)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break;
            }

            for node in &to_query {
                queried.insert(node.identity.node_id);
                let peer = PeerAddr {
                    node_id: Some(node.identity.node_id),
                    addr: node.addr,
                };
                let msg = Message::FindNode { target: *target };
                if let Ok(Some(Message::FindNodeResponse { nodes })) = self.rpc(&peer, &msg).await {
                    let mut verified = Vec::new();
                    for new_node in nodes {
                        if pow::verify_pow(&new_node.identity)
                            && new_node.identity.node_id != self.node_id()
                        {
                            verified.push(new_node);
                        }
                    }
                    // Update routing table with discovered nodes
                    {
                        let mut rt = self.routing.lock().await;
                        for node in &verified {
                            rt.update(node.clone());
                        }
                    }
                    for new_node in verified {
                        if !closest
                            .iter()
                            .any(|n| n.identity.node_id == new_node.identity.node_id)
                        {
                            closest.push(new_node);
                        }
                    }
                }
            }

            // Sort by distance and trim to k
            closest.sort_by(|a, b| {
                let da = distance::xor_distance(target, &a.identity.node_id);
                let db = distance::xor_distance(target, &b.identity.node_id);
                da.cmp(&db)
            });
            closest.truncate(k);
        }

        closest
    }

    /// Publish a tessera pointer to the k closest nodes.
    pub async fn publish(&self, pointer: TesseraPointer) -> Result<usize, DhtError> {
        // Truncate hash to NodeId for lookup
        let mut target_bytes = [0u8; 20];
        target_bytes.copy_from_slice(&pointer.tessera_hash.as_bytes()[..20]);
        let target = NodeId::new(target_bytes);

        let closest = self.find_closest_nodes(&target).await;
        if closest.is_empty() {
            return Err(DhtError::PublishFailed { got: 0, needed: 1 });
        }

        let mut acks = 0;
        for node in &closest {
            let peer = PeerAddr {
                node_id: Some(node.identity.node_id),
                addr: node.addr,
            };
            let msg = Message::Store {
                key: pointer.tessera_hash,
                pointer: pointer.clone(),
            };
            if let Ok(Some(Message::StoreResponse { accepted: true })) = self.rpc(&peer, &msg).await
            {
                acks += 1;
            }
        }

        Ok(acks)
    }

    /// Find a tessera pointer by content hash.
    pub async fn find_tessera(
        &self,
        hash: &ContentHash,
    ) -> Result<Option<TesseraPointer>, DhtError> {
        // Check local store first
        if let Some(ptr) = self.store.lock().await.get(hash) {
            return Ok(Some(ptr.clone()));
        }

        // Truncate hash to NodeId for lookup
        let mut target_bytes = [0u8; 20];
        target_bytes.copy_from_slice(&hash.as_bytes()[..20]);
        let target = NodeId::new(target_bytes);

        let k = self.config.k;
        let alpha = self.config.alpha;

        let mut closest: Vec<NodeInfo> = self.routing.lock().await.closest(&target, k);
        let mut queried: HashSet<NodeId> = HashSet::new();

        loop {
            let to_query: Vec<NodeInfo> = closest
                .iter()
                .filter(|n| !queried.contains(&n.identity.node_id))
                .take(alpha)
                .cloned()
                .collect();

            if to_query.is_empty() {
                break;
            }

            for node in &to_query {
                queried.insert(node.identity.node_id);
                let peer = PeerAddr {
                    node_id: Some(node.identity.node_id),
                    addr: node.addr,
                };
                let msg = Message::FindValue { key: *hash };
                match self.rpc(&peer, &msg).await {
                    Ok(Some(Message::FindValueResponse {
                        result: FindValueResult::Found(ptr),
                    })) => {
                        return Ok(Some(ptr));
                    }
                    Ok(Some(Message::FindValueResponse {
                        result: FindValueResult::Nodes(nodes),
                    })) => {
                        let mut verified = Vec::new();
                        for new_node in nodes {
                            if pow::verify_pow(&new_node.identity)
                                && new_node.identity.node_id != self.node_id()
                            {
                                verified.push(new_node);
                            }
                        }
                        {
                            let mut rt = self.routing.lock().await;
                            for node in &verified {
                                rt.update(node.clone());
                            }
                        }
                        for new_node in verified {
                            if !closest
                                .iter()
                                .any(|n| n.identity.node_id == new_node.identity.node_id)
                            {
                                closest.push(new_node);
                            }
                        }
                    }
                    _ => {}
                }
            }

            closest.sort_by(|a, b| {
                let da = distance::xor_distance(&target, &a.identity.node_id);
                let db = distance::xor_distance(&target, &b.identity.node_id);
                da.cmp(&db)
            });
            closest.truncate(k);
        }

        Ok(None)
    }

    /// Bootstrap by connecting to seed addresses and doing a self-lookup.
    pub async fn bootstrap(&self, seeds: &[SocketAddr]) -> Result<(), DhtError> {
        let mut any_success = false;

        for &addr in seeds {
            if self.ping(addr).await {
                any_success = true;
            }
        }

        if !any_success {
            return Err(DhtError::BootstrapFailed);
        }

        // Do a lookup for our own ID to populate our routing table
        self.find_closest_nodes(&self.node_id()).await;

        Ok(())
    }

    /// Get the number of known peers.
    pub async fn routing_table_size(&self) -> usize {
        self.routing.lock().await.len()
    }

    /// Get the number of stored pointers.
    pub async fn store_size(&self) -> usize {
        self.store.lock().await.len()
    }

    /// Run the engine's main loop: receive messages, run maintenance timers.
    pub async fn run(&self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut refresh_interval = tokio::time::interval(self.config.bucket_refresh_interval);
        let mut republish_interval = tokio::time::interval(self.config.republish_interval);
        let mut stale_interval = tokio::time::interval(self.config.stale_check_interval);

        // Skip the first immediate tick
        refresh_interval.tick().await;
        republish_interval.tick().await;
        stale_interval.tick().await;

        loop {
            tokio::select! {
                msg = self.transport.recv() => {
                    match msg {
                        Ok(envelope) => self.handle_envelope(envelope).await,
                        Err(_) => break,
                    }
                }
                _ = refresh_interval.tick() => {
                    self.refresh_buckets().await;
                }
                _ = republish_interval.tick() => {
                    self.republish_pointers().await;
                }
                _ = stale_interval.tick() => {
                    self.check_stale_contacts().await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("DHT engine shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Refresh buckets by doing a random lookup in each bucket's range.
    async fn refresh_buckets(&self) {
        let target = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mut target_bytes = [0u8; 20];
            rng.fill(&mut target_bytes);
            NodeId::new(target_bytes)
        };
        self.find_closest_nodes(&target).await;
    }

    /// Republish all stored pointers to their closest nodes.
    async fn republish_pointers(&self) {
        let pointers = self.store.lock().await.pointers();
        for ptr in pointers {
            let _ = self.publish(ptr).await;
        }
    }

    /// Check for stale contacts by pinging random nodes.
    async fn check_stale_contacts(&self) {
        let target = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mut target_bytes = [0u8; 20];
            rng.fill(&mut target_bytes);
            NodeId::new(target_bytes)
        };
        let nodes = self.routing.lock().await.closest(&target, 3);
        for node in nodes {
            if !self.ping(node.addr).await {
                self.routing.lock().await.remove(&node.identity.node_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_net::SimNetwork;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    async fn create_engine(net: &SimNetwork, port: u16) -> Arc<DhtEngine> {
        let transport = net.create_transport(addr(port), 256).await;
        let pubkey = [port as u8; 32];
        let identity = pow::generate_node_identity(&pubkey);
        DhtEngine::new(identity, Box::new(transport), DhtConfig::default())
    }

    #[tokio::test]
    async fn handle_ping_returns_pong() {
        let net = SimNetwork::new();
        let e1 = create_engine(&net, 100).await;
        let e2 = create_engine(&net, 101).await;

        // e1 sends Ping to e2, e2 handles it
        let ping = Message::Ping {
            sender: e1.identity().clone(),
        };
        let peer = PeerAddr {
            node_id: None,
            addr: addr(100),
        };
        let response = e2.handle_message(&ping, &peer).await;

        match response {
            Some(Message::Pong {
                sender,
                capabilities,
            }) => {
                assert_eq!(sender.node_id, e2.node_id());
                assert!(capabilities.has(Capabilities::PING));
            }
            _ => panic!("expected Pong"),
        }
    }

    #[tokio::test]
    async fn handle_ping_adds_to_routing_table() {
        let net = SimNetwork::new();
        let e1 = create_engine(&net, 200).await;
        let e2 = create_engine(&net, 201).await;

        let ping = Message::Ping {
            sender: e1.identity().clone(),
        };
        let peer = PeerAddr {
            node_id: Some(e1.node_id()),
            addr: addr(200),
        };

        assert_eq!(e2.routing_table_size().await, 0);
        e2.handle_message(&ping, &peer).await;
        assert_eq!(e2.routing_table_size().await, 1);
    }

    #[tokio::test]
    async fn handle_find_node_returns_closest() {
        let net = SimNetwork::new();
        let engine = create_engine(&net, 300).await;

        // Add some nodes to routing table
        let node1 = pow::generate_node_identity(&[0x01; 32]);
        let node2 = pow::generate_node_identity(&[0x02; 32]);
        {
            let mut rt = engine.routing.lock().await;
            rt.update(NodeInfo {
                identity: node1.clone(),
                addr: addr(301),
                capabilities: Capabilities::phase1_default(),
            });
            rt.update(NodeInfo {
                identity: node2.clone(),
                addr: addr(302),
                capabilities: Capabilities::phase1_default(),
            });
        }

        let target = NodeId::new([0xff; 20]);
        let msg = Message::FindNode { target };
        let peer = PeerAddr {
            node_id: None,
            addr: addr(399),
        };
        let response = engine.handle_message(&msg, &peer).await;

        match response {
            Some(Message::FindNodeResponse { nodes }) => {
                assert_eq!(nodes.len(), 2);
            }
            _ => panic!("expected FindNodeResponse"),
        }
    }

    #[tokio::test]
    async fn handle_store_and_find_value() {
        let net = SimNetwork::new();
        let engine = create_engine(&net, 400).await;

        // Store a pointer
        let ptr = TesseraPointer {
            tessera_hash: ContentHash::new([0xaa; 32]),
            size_bytes: 1000,
            holders: vec![],
            visibility: Visibility::Public,
            created_at: chrono::Utc::now(),
        };
        let store_msg = Message::Store {
            key: ptr.tessera_hash,
            pointer: ptr.clone(),
        };
        let peer = PeerAddr {
            node_id: None,
            addr: addr(499),
        };
        let resp = engine.handle_message(&store_msg, &peer).await;
        assert!(matches!(
            resp,
            Some(Message::StoreResponse { accepted: true })
        ));

        // Find it back
        let find_msg = Message::FindValue {
            key: ContentHash::new([0xaa; 32]),
        };
        let resp = engine.handle_message(&find_msg, &peer).await;
        match resp {
            Some(Message::FindValueResponse {
                result: FindValueResult::Found(found_ptr),
            }) => {
                assert_eq!(found_ptr.tessera_hash, ptr.tessera_hash);
            }
            _ => panic!("expected Found"),
        }
    }

    #[tokio::test]
    async fn handle_find_value_returns_nodes_when_not_found() {
        let net = SimNetwork::new();
        let engine = create_engine(&net, 500).await;

        let find_msg = Message::FindValue {
            key: ContentHash::new([0xbb; 32]),
        };
        let peer = PeerAddr {
            node_id: None,
            addr: addr(599),
        };
        let resp = engine.handle_message(&find_msg, &peer).await;
        assert!(matches!(
            resp,
            Some(Message::FindValueResponse {
                result: FindValueResult::Nodes(_)
            })
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_pow() {
        let net = SimNetwork::new();
        let engine = create_engine(&net, 600).await;

        // Create identity with bad PoW
        let bad_identity = NodeIdentity {
            node_id: NodeId::new([0xff; 20]), // won't match BLAKE3(key||nonce)
            public_key: [0x01; 32],
            nonce: 0,
        };
        let ping = Message::Ping {
            sender: bad_identity,
        };
        let peer = PeerAddr {
            node_id: None,
            addr: addr(601),
        };

        engine.handle_message(&ping, &peer).await;
        // Should not be added to routing table
        assert_eq!(engine.routing_table_size().await, 0);
    }

    #[tokio::test]
    async fn ping_rpc_roundtrip() {
        let net = SimNetwork::new();
        let e1 = create_engine(&net, 700).await;
        let e2 = create_engine(&net, 701).await;

        // Spawn e2's message handler (handles the Ping request, sends Pong)
        let e2_clone = Arc::clone(&e2);
        let handler = tokio::spawn(async move {
            let envelope = e2_clone.transport.recv().await.unwrap();
            e2_clone.handle_envelope(envelope).await;
        });

        // Spawn a recv loop for e1 so the Pong response gets routed to the pending RPC
        let e1_clone = Arc::clone(&e1);
        let e1_recv = tokio::spawn(async move {
            let envelope = e1_clone.transport.recv().await.unwrap();
            e1_clone.handle_envelope(envelope).await;
        });

        // e1 pings e2
        let result = e1.ping(addr(701)).await;
        assert!(result);

        handler.await.unwrap();
        e1_recv.await.unwrap();

        // e1 should now have e2 in its routing table
        assert_eq!(e1.routing_table_size().await, 1);
    }

    #[tokio::test]
    async fn engine_run_processes_messages() {
        let net = SimNetwork::new();
        let e1 = create_engine(&net, 800).await;
        let e2 = create_engine(&net, 801).await;

        // Start both engines' run loops so responses get routed correctly
        let (shutdown_tx1, shutdown_rx1) = tokio::sync::watch::channel(false);
        let e1_clone = Arc::clone(&e1);
        let run_handle1 = tokio::spawn(async move {
            e1_clone.run(shutdown_rx1).await;
        });

        let (shutdown_tx2, shutdown_rx2) = tokio::sync::watch::channel(false);
        let e2_clone = Arc::clone(&e2);
        let run_handle2 = tokio::spawn(async move {
            e2_clone.run(shutdown_rx2).await;
        });

        // e1 pings e2 (handled by e2's run loop, response routed by e1's run loop)
        let result = e1.ping(addr(801)).await;
        assert!(result);

        // e1 should know about e2
        assert_eq!(e1.routing_table_size().await, 1);

        // Shutdown both engines
        shutdown_tx1.send(true).unwrap();
        shutdown_tx2.send(true).unwrap();
        run_handle1.await.unwrap();
        run_handle2.await.unwrap();
    }
}
