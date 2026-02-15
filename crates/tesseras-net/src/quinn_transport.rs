//! QUIC transport using quinn. Self-signed TLS, connection pooling,
//! one bidirectional stream per RPC with length-prefix framing.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;
use quinn::{Endpoint, RecvStream, SendStream};
use tokio::sync::mpsc;

use crate::codec::{self, ALPN_TESSERAS_V1};
use crate::error::NetError;
use crate::transport::{Envelope, PeerAddr, Transport};

/// Configuration for the QUIC connection pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of pooled connections. Default: 256.
    pub max_connections: usize,
    /// Idle timeout before a connection is reaped. Default: 5 minutes.
    pub idle_timeout: Duration,
    /// How often the reaper task runs. Default: 30 seconds.
    pub reaper_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 256,
            idle_timeout: Duration::from_secs(300),
            reaper_interval: Duration::from_secs(30),
        }
    }
}

struct PooledConnection {
    conn: quinn::Connection,
    last_used: Instant,
}

/// QUIC transport implementation using quinn.
///
/// Supports binding to multiple addresses (e.g. one IPv4 and one IPv6) via
/// `bind_multiple`. Outbound connections pick the endpoint matching the
/// target address family.
pub struct QuinnTransport {
    endpoints: Vec<Endpoint>,
    connections: Arc<DashMap<SocketAddr, PooledConnection>>,
    pool_config: PoolConfig,
    incoming_tx: mpsc::Sender<Envelope>,
    incoming_rx: tokio::sync::Mutex<mpsc::Receiver<Envelope>>,
}

impl QuinnTransport {
    /// Bind to a single address and start accepting connections.
    pub async fn bind(addr: SocketAddr) -> Result<Arc<Self>, NetError> {
        Self::bind_multiple(&[addr]).await
    }

    /// Bind to a single address with a custom pool configuration.
    pub async fn bind_with_config(
        addr: SocketAddr,
        pool_config: PoolConfig,
    ) -> Result<Arc<Self>, NetError> {
        Self::bind_multiple_with_config(&[addr], pool_config).await
    }

    /// Bind to multiple addresses (e.g. IPv4 + IPv6) and start accepting
    /// connections on each endpoint.
    pub async fn bind_multiple(addrs: &[SocketAddr]) -> Result<Arc<Self>, NetError> {
        Self::bind_multiple_with_config(addrs, PoolConfig::default()).await
    }

    /// Bind to multiple addresses with a custom pool configuration.
    pub async fn bind_multiple_with_config(
        addrs: &[SocketAddr],
        pool_config: PoolConfig,
    ) -> Result<Arc<Self>, NetError> {
        if addrs.is_empty() {
            return Err(NetError::ConnectionFailed(
                "at least one listen address required".to_string(),
            ));
        }

        let (server_config, _client_config) = Self::make_tls_configs()?;

        let mut endpoints = Vec::with_capacity(addrs.len());
        for &addr in addrs {
            let ep = Endpoint::server(server_config.clone(), addr)
                .map_err(|e| NetError::ConnectionFailed(e.to_string()))?;
            endpoints.push(ep);
        }

        let (incoming_tx, incoming_rx) = mpsc::channel(1024);
        let connections = Arc::new(DashMap::new());

        let transport = Arc::new(Self {
            endpoints,
            connections: Arc::clone(&connections),
            pool_config: pool_config.clone(),
            incoming_tx,
            incoming_rx: tokio::sync::Mutex::new(incoming_rx),
        });

        // Spawn an accept loop per endpoint
        for i in 0..transport.endpoints.len() {
            let t = Arc::clone(&transport);
            tokio::spawn(async move {
                t.accept_loop(i).await;
            });
        }

        // Spawn idle connection reaper
        Self::spawn_reaper(connections, pool_config);

        Ok(transport)
    }

    fn spawn_reaper(connections: Arc<DashMap<SocketAddr, PooledConnection>>, config: PoolConfig) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.reaper_interval);
            loop {
                interval.tick().await;
                let now = Instant::now();
                connections.retain(|_, pooled| {
                    if pooled.conn.close_reason().is_some() {
                        return false; // already dead
                    }
                    if now.duration_since(pooled.last_used) > config.idle_timeout {
                        pooled.conn.close(0u32.into(), b"idle");
                        return false;
                    }
                    true
                });
            }
        });
    }

    /// Generate self-signed TLS configuration.
    fn make_tls_configs() -> Result<(quinn::ServerConfig, quinn::ClientConfig), NetError> {
        let cert = rcgen::generate_simple_self_signed(vec!["tesseras".to_string()])
            .map_err(|e| NetError::ConnectionFailed(e.to_string()))?;

        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
        let key_der = rustls::pki_types::PrivateKeyDer::try_from(cert.key_pair.serialize_der())
            .map_err(|e| NetError::ConnectionFailed(e.to_string()))?;

        // Server config
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der.clone_key())
            .map_err(|e| NetError::ConnectionFailed(e.to_string()))?;
        server_crypto.alpn_protocols = vec![ALPN_TESSERAS_V1.to_vec()];
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .map_err(|e| NetError::ConnectionFailed(e.to_string()))?,
        ));

        // Client config (skip server cert verification for self-signed)
        let mut client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();
        client_crypto.alpn_protocols = vec![ALPN_TESSERAS_V1.to_vec()];
        let client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
                .map_err(|e| NetError::ConnectionFailed(e.to_string()))?,
        ));

        Ok((server_config, client_config))
    }

    /// Accept loop for a specific endpoint.
    async fn accept_loop(&self, endpoint_idx: usize) {
        let endpoint = &self.endpoints[endpoint_idx];
        while let Some(incoming) = endpoint.accept().await {
            let tx = self.incoming_tx.clone();
            let connections = &self.connections;
            let conn = match incoming.await {
                Ok(c) => c,
                Err(e) => {
                    tracing::debug!("failed to accept connection: {e}");
                    continue;
                }
            };
            let remote = conn.remote_address();
            connections.insert(
                remote,
                PooledConnection {
                    conn: conn.clone(),
                    last_used: Instant::now(),
                },
            );

            // Spawn a task to handle streams from this connection
            tokio::spawn(async move {
                while let Ok((send_stream, mut recv)) = conn.accept_bi().await {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        if let Ok(data) = read_message(&mut recv).await {
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            let _ = tx
                                .send(Envelope {
                                    peer: PeerAddr {
                                        node_id: None,
                                        addr: remote,
                                    },
                                    payload: data,
                                    response_tx: Some(resp_tx),
                                })
                                .await;
                            // Wait for the response and write it back on the same stream
                            let mut send = send_stream;
                            if let Ok(response_data) = resp_rx.await {
                                let _ = write_message(&mut send, &response_data).await;
                                let _ = send.finish();
                            }
                        }
                    });
                }
            });
        }
    }

    /// Pick the endpoint whose address family matches the target.
    /// Falls back to the first endpoint if no match is found.
    fn endpoint_for(&self, addr: &SocketAddr) -> &Endpoint {
        self.endpoints
            .iter()
            .find(|ep| {
                ep.local_addr()
                    .map(|a| a.is_ipv4() == addr.is_ipv4())
                    .unwrap_or(false)
            })
            .unwrap_or(&self.endpoints[0])
    }

    /// Get or create a connection to a peer.
    async fn get_connection(&self, addr: SocketAddr) -> Result<quinn::Connection, NetError> {
        if let Some(mut entry) = self.connections.get_mut(&addr) {
            if entry.conn.close_reason().is_none() {
                entry.last_used = Instant::now();
                return Ok(entry.conn.clone());
            }
        }

        // Evict LRU if at capacity
        if self.connections.len() >= self.pool_config.max_connections {
            let lru_key = self
                .connections
                .iter()
                .min_by_key(|entry| entry.last_used)
                .map(|entry| *entry.key());
            if let Some(key) = lru_key {
                if let Some((_, evicted)) = self.connections.remove(&key) {
                    evicted.conn.close(0u32.into(), b"pool-full");
                }
            }
        }

        let (_, client_config) = Self::make_tls_configs()?;
        let endpoint = self.endpoint_for(&addr);
        let conn = endpoint
            .connect_with(client_config, addr, "tesseras")
            .map_err(|e| NetError::ConnectionFailed(e.to_string()))?
            .await
            .map_err(|e| NetError::ConnectionFailed(e.to_string()))?;

        self.connections.insert(
            addr,
            PooledConnection {
                conn: conn.clone(),
                last_used: Instant::now(),
            },
        );
        Ok(conn)
    }
}

#[async_trait]
impl Transport for QuinnTransport {
    async fn send(&self, peer: &PeerAddr, data: &[u8]) -> Result<(), NetError> {
        let conn = self.get_connection(peer.addr).await?;
        let (mut send, recv_stream) = conn
            .open_bi()
            .await
            .map_err(|e| NetError::SendFailed(e.to_string()))?;

        write_message(&mut send, data).await?;
        send.finish()
            .map_err(|e| NetError::SendFailed(e.to_string()))?;

        // Read the response asynchronously so we don't block the caller.
        // The server writes its response on the same bidirectional stream.
        let tx = self.incoming_tx.clone();
        let peer_addr = peer.addr;
        let node_id = peer.node_id;
        tokio::spawn(async move {
            let mut recv = recv_stream;
            if let Ok(response_data) = read_message(&mut recv).await {
                let _ = tx
                    .send(Envelope {
                        peer: PeerAddr {
                            node_id,
                            addr: peer_addr,
                        },
                        payload: response_data,
                        response_tx: None,
                    })
                    .await;
            }
        });

        Ok(())
    }

    async fn recv(&self) -> Result<Envelope, NetError> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await.ok_or(NetError::Closed)
    }

    async fn disconnect(&self, peer: &PeerAddr) {
        if let Some((_, pooled)) = self.connections.remove(&peer.addr) {
            pooled.conn.close(0u32.into(), b"disconnect");
        }
    }

    fn local_addrs(&self) -> Vec<SocketAddr> {
        self.endpoints
            .iter()
            .map(|ep| ep.local_addr().unwrap())
            .collect()
    }

    fn local_addr(&self) -> SocketAddr {
        self.endpoints[0].local_addr().unwrap()
    }
}

/// Write a length-prefixed message to a QUIC send stream.
async fn write_message(send: &mut SendStream, data: &[u8]) -> Result<(), NetError> {
    let len = (data.len() as u32).to_be_bytes();
    send.write_all(&len)
        .await
        .map_err(|e| NetError::SendFailed(e.to_string()))?;
    send.write_all(data)
        .await
        .map_err(|e| NetError::SendFailed(e.to_string()))?;
    Ok(())
}

/// Read a length-prefixed message from a QUIC recv stream.
async fn read_message(recv: &mut RecvStream) -> Result<Vec<u8>, NetError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetError::ReceiveFailed(e.to_string()))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > codec::MAX_MESSAGE_SIZE {
        return Err(NetError::MessageTooLarge {
            size: len,
            max: codec::MAX_MESSAGE_SIZE,
        });
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(|e| NetError::ReceiveFailed(e.to_string()))?;
    Ok(buf)
}

/// Skip server certificate verification (for self-signed certs in P2P).
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quinn_bind_and_local_addr() {
        let t = QuinnTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        assert_ne!(t.local_addr().port(), 0);
    }

    #[tokio::test]
    async fn quinn_bind_ipv6_loopback() {
        let t = QuinnTransport::bind("[::1]:0".parse().unwrap())
            .await
            .unwrap();
        assert!(t.local_addr().is_ipv6());
        assert_ne!(t.local_addr().port(), 0);
    }

    #[tokio::test]
    async fn quinn_send_recv_ipv6() {
        let t1 = QuinnTransport::bind("[::1]:0".parse().unwrap())
            .await
            .unwrap();
        let t2 = QuinnTransport::bind("[::1]:0".parse().unwrap())
            .await
            .unwrap();

        let peer = PeerAddr {
            node_id: None,
            addr: t2.local_addr(),
        };

        t1.send(&peer, b"hello ipv6").await.unwrap();
        let env = t2.recv().await.unwrap();
        assert_eq!(env.payload, b"hello ipv6");
        assert!(env.peer.addr.is_ipv6());
    }

    #[tokio::test]
    async fn quinn_send_recv() {
        let t1 = QuinnTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let t2 = QuinnTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        let peer = PeerAddr {
            node_id: None,
            addr: t2.local_addr(),
        };

        t1.send(&peer, b"hello quic").await.unwrap();
        let env = t2.recv().await.unwrap();
        assert_eq!(env.payload, b"hello quic");
    }

    #[tokio::test]
    async fn quinn_bind_multiple_v4_v6() {
        let t = QuinnTransport::bind_multiple(&[
            "127.0.0.1:0".parse().unwrap(),
            "[::1]:0".parse().unwrap(),
        ])
        .await
        .unwrap();

        let addrs = t.local_addrs();
        assert_eq!(addrs.len(), 2);
        assert!(addrs[0].is_ipv4());
        assert!(addrs[1].is_ipv6());
    }

    #[tokio::test]
    async fn quinn_cross_family_connect() {
        // Dual-stack transport listening on both v4 and v6
        let dual = QuinnTransport::bind_multiple(&[
            "127.0.0.1:0".parse().unwrap(),
            "[::1]:0".parse().unwrap(),
        ])
        .await
        .unwrap();

        // v4-only transport
        let v4 = QuinnTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();

        // v6-only transport
        let v6 = QuinnTransport::bind("[::1]:0".parse().unwrap())
            .await
            .unwrap();

        // v4 sends to dual-stack's v4 endpoint
        let dual_addrs = dual.local_addrs();
        let peer_v4 = PeerAddr {
            node_id: None,
            addr: dual_addrs[0], // v4 addr
        };
        v4.send(&peer_v4, b"from v4").await.unwrap();
        let env = dual.recv().await.unwrap();
        assert_eq!(env.payload, b"from v4");

        // v6 sends to dual-stack's v6 endpoint
        let peer_v6 = PeerAddr {
            node_id: None,
            addr: dual_addrs[1], // v6 addr
        };
        v6.send(&peer_v6, b"from v6").await.unwrap();
        let env = dual.recv().await.unwrap();
        assert_eq!(env.payload, b"from v6");
    }
}
