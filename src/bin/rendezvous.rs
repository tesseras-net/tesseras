//
// Copyright (c) 2025 murilo ijanc' <murilo@ijanc.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//

// https://en.wikipedia.org/wiki/Rendezvous_protocol

use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use bincode::{Decode, Encode};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct PeerInfo {
    peer_id: String,
    public_addr: SocketAddr,
    private_addr: Option<SocketAddr>,
    last_seen: SystemTime,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub enum RendezvousMessage {
    Register { peer_id: String, private_addr: SocketAddr },
    Query { target_peer_id: String },
    PeerInfo { peer: PeerInfo },
    InitiateConnection { from_peer_id: String, to_peer_id: String },
}

/// RendezvousServer
///
/// A rendezvous protocol is a computer network protocol that enables resources
/// or P2P network peers to find each other. A rendezvous protocol uses a
/// handshaking model, unlike an eager protocol which directly copies the data
pub struct RendezvousServer {
    socket: UdpSocket,
    peers: HashMap<String, PeerInfo>,
}

impl RendezvousServer {
    pub fn new(bind_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;

        info!("Server Rendezvous Listening on {}", bind_addr);

        Ok(RendezvousServer { socket, peers: HashMap::new() })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = bincode::config::standard();
        let mut buf = [0u8; 65536];

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, peer_addr)) => {
                    if let Ok((msg, _)) =
                        bincode::decode_from_slice(&buf[..len], config)
                    {
                        self.handle_message(msg, peer_addr)?;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => error!("Erro: {}", e),
            }
        }
    }

    fn handle_message(
        &mut self,
        msg: RendezvousMessage,
        from: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = bincode::config::standard();
        match msg {
            RendezvousMessage::Register { peer_id, private_addr } => {
                debug!(
                    "Peer {} registrado: público={}, privado={}",
                    peer_id, from, private_addr
                );

                self.peers.insert(
                    peer_id.clone(),
                    PeerInfo {
                        peer_id,
                        public_addr: from, // Address stun
                        private_addr: Some(private_addr),
                        last_seen: SystemTime::now(),
                    },
                );
            }

            RendezvousMessage::Query { target_peer_id } => {
                if let Some(peer_info) = self.peers.get(&target_peer_id) {
                    let response = RendezvousMessage::PeerInfo {
                        peer: peer_info.clone(),
                    };

                    self.socket.send_to(
                        &bincode::encode_to_vec(&response, config)?,
                        from,
                    )?;
                }
            }

            RendezvousMessage::InitiateConnection {
                from_peer_id,
                to_peer_id,
            } => {
                // Notify peers
                if let (Some(from_peer), Some(to_peer)) = (
                    self.peers.get(&from_peer_id),
                    self.peers.get(&to_peer_id),
                ) {
                    // Send info from B to A
                    let msg_to_a =
                        RendezvousMessage::PeerInfo { peer: to_peer.clone() };
                    self.socket.send_to(
                        &bincode::encode_to_vec(&msg_to_a, config)?,
                        from_peer.public_addr,
                    )?;

                    // Send info from A to B
                    let msg_to_b = RendezvousMessage::PeerInfo {
                        peer: from_peer.clone(),
                    };
                    self.socket.send_to(
                        &bincode::encode_to_vec(&msg_to_b, config)?,
                        to_peer.public_addr,
                    )?;

                    debug!(
                        "Iniciando hole punching: {} <-> {}",
                        from_peer_id, to_peer_id
                    );
                }
            }

            _ => {}
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().format_timestamp(None).init();

    let mut server = RendezvousServer::new("0.0.0.0:8000")?;
    server.run()
}
