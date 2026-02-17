use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use quinn::{Endpoint, ServerConfig};
use tracing::{debug, warn};

use crate::dht::DhtMessage;

/// Generate a self-signed TLS certificate for QUIC transport.
fn generate_self_signed_cert() -> Result<
    (
        rustls::pki_types::CertificateDer<'static>,
        rustls::pki_types::PrivateKeyDer<'static>,
    ),
    NetError,
> {
    let cert = rcgen::generate_simple_self_signed(vec!["tesseras".into()])
        .map_err(|e| NetError::Tls(e.to_string()))?;
    let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
    let key_der = rustls::pki_types::PrivateKeyDer::from(
        rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
    );
    Ok((cert_der, key_der))
}

/// Create a QUIC server config with self-signed cert.
fn make_server_config() -> Result<ServerConfig, NetError> {
    let (cert, key) = generate_self_signed_cert()?;

    let mut server_crypto = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| NetError::Tls(e.to_string()))?
    .with_no_client_auth()
    .with_single_cert(vec![cert], key)
    .map_err(|e| NetError::Tls(e.to_string()))?;
    server_crypto.alpn_protocols = vec![b"tesseras/1".to_vec()];

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| NetError::Tls(e.to_string()))?,
    ));
    Ok(server_config)
}

/// Create a QUIC client config that accepts any certificate (self-signed peers).
fn make_client_config() -> Result<quinn::ClientConfig, NetError> {
    let mut client_crypto = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| NetError::Tls(e.to_string()))?
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
    .with_no_client_auth();
    client_crypto.alpn_protocols = vec![b"tesseras/1".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| NetError::Tls(e.to_string()))?,
    ));
    Ok(client_config)
}

/// Skip server certificate verification (peers use self-signed certs).
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

/// QUIC transport for tesseras network.
pub struct QuicTransport {
    endpoint: Endpoint,
}

impl QuicTransport {
    /// Create a new QUIC transport listening on the given address.
    /// The endpoint supports both accepting incoming and initiating outgoing connections.
    pub async fn bind(listen_addr: SocketAddr) -> Result<Self, NetError> {
        let server_config = make_server_config()?;
        let mut endpoint = Endpoint::server(server_config, listen_addr)
            .map_err(|e| NetError::Bind(e.to_string()))?;
        endpoint.set_default_client_config(make_client_config()?);
        Ok(Self { endpoint })
    }

    /// Create a client-only transport (no listening).
    pub fn client() -> Result<Self, NetError> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| NetError::Bind(e.to_string()))?;
        endpoint.set_default_client_config(make_client_config()?);
        Ok(Self { endpoint })
    }

    /// Get the local address this transport is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.endpoint
            .local_addr()
            .map_err(|e| NetError::Bind(e.to_string()))
    }

    /// Accept an incoming connection.
    pub async fn accept(&self) -> Result<quinn::Connection, NetError> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or(NetError::ConnectionClosed)?;
        let conn = incoming
            .await
            .map_err(|e| NetError::Connection(e.to_string()))?;
        Ok(conn)
    }

    /// Connect to a peer.
    pub async fn connect(&self, addr: SocketAddr) -> Result<quinn::Connection, NetError> {
        let conn = self
            .endpoint
            .connect(addr, "tesseras")
            .map_err(|e| NetError::Connection(e.to_string()))?
            .await
            .map_err(|e| NetError::Connection(e.to_string()))?;
        Ok(conn)
    }

    /// Close the transport.
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"shutdown");
    }
}

/// Send a length-prefixed MessagePack message over a QUIC send stream.
pub async fn send_message(send: &mut quinn::SendStream, msg: &DhtMessage) -> Result<(), NetError> {
    let data = msg.to_bytes();
    let len = (data.len() as u32).to_be_bytes();
    send.write_all(&len)
        .await
        .map_err(|e| NetError::Write(e.to_string()))?;
    send.write_all(&data)
        .await
        .map_err(|e| NetError::Write(e.to_string()))?;
    Ok(())
}

/// Receive a length-prefixed MessagePack message from a QUIC receive stream.
pub async fn receive_message(recv: &mut quinn::RecvStream) -> Result<DhtMessage, NetError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetError::Read(e.to_string()))?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 16 * 1024 * 1024 {
        return Err(NetError::MessageTooLarge(len));
    }

    let mut data = vec![0u8; len];
    recv.read_exact(&mut data)
        .await
        .map_err(|e| NetError::Read(e.to_string()))?;

    DhtMessage::from_bytes(&data).map_err(|e| NetError::Deserialize(e.to_string()))
}

/// Stream a blob over a dedicated QUIC send stream.
pub async fn stream_blob(send: &mut quinn::SendStream, data: &[u8]) -> Result<(), NetError> {
    let len = (data.len() as u64).to_be_bytes();
    send.write_all(&len)
        .await
        .map_err(|e| NetError::Write(e.to_string()))?;
    send.write_all(data)
        .await
        .map_err(|e| NetError::Write(e.to_string()))?;
    send.finish().map_err(|e| NetError::Write(e.to_string()))?;
    Ok(())
}

/// Receive a blob from a dedicated QUIC receive stream.
pub async fn receive_blob(recv: &mut quinn::RecvStream) -> Result<Vec<u8>, NetError> {
    let mut len_buf = [0u8; 8];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetError::Read(e.to_string()))?;
    let len = u64::from_be_bytes(len_buf) as usize;

    if len > 1024 * 1024 * 1024 {
        return Err(NetError::MessageTooLarge(len));
    }

    let mut data = vec![0u8; len];
    recv.read_exact(&mut data)
        .await
        .map_err(|e| NetError::Read(e.to_string()))?;

    Ok(data)
}

// --- STUN client ---

/// STUN message type: Binding Request.
const STUN_BINDING_REQUEST: u16 = 0x0001;

/// STUN magic cookie (RFC 5389).
const STUN_MAGIC_COOKIE: u32 = 0x2112_A442;

/// STUN attribute types.
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;

/// Discover external address via STUN Binding Request.
/// Tries each STUN server in order, returns the first successful result.
pub async fn discover_external_addr(stun_servers: &[String]) -> Option<SocketAddr> {
    for server in stun_servers {
        match stun_binding_request(server).await {
            Ok(addr) => {
                debug!("STUN discovered external address: {addr} (via {server})");
                return Some(addr);
            }
            Err(e) => {
                debug!("STUN request to {server} failed: {e}");
                continue;
            }
        }
    }
    warn!("STUN: could not discover external address from any server");
    None
}

/// Send a STUN Binding Request and parse the response.
async fn stun_binding_request(server: &str) -> Result<SocketAddr, NetError> {
    use tokio::net::UdpSocket;

    let addr: SocketAddr = tokio::net::lookup_host(server)
        .await
        .map_err(|e| NetError::Stun(format!("DNS resolve {server}: {e}")))?
        .next()
        .ok_or_else(|| NetError::Stun(format!("no addresses for {server}")))?;

    let bind_addr: SocketAddr = if addr.is_ipv6() {
        SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0)
    } else {
        SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0)
    };

    let socket = UdpSocket::bind(bind_addr)
        .await
        .map_err(|e| NetError::Stun(e.to_string()))?;

    // Build STUN Binding Request (20 bytes)
    let mut txn_id = [0u8; 12];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut txn_id);

    let mut request = [0u8; 20];
    request[0..2].copy_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());
    request[2..4].copy_from_slice(&0u16.to_be_bytes()); // message length = 0
    request[4..8].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
    request[8..20].copy_from_slice(&txn_id);

    socket
        .send_to(&request, addr)
        .await
        .map_err(|e| NetError::Stun(e.to_string()))?;

    // Wait for response (max 3 seconds)
    let mut buf = [0u8; 512];
    let len = match tokio::time::timeout(std::time::Duration::from_secs(3), socket.recv(&mut buf))
        .await
    {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(NetError::Stun(e.to_string())),
        Err(_) => return Err(NetError::Stun("timeout".into())),
    };

    if len < 20 {
        return Err(NetError::Stun("response too short".into()));
    }

    // Verify transaction ID matches
    if buf[8..20] != txn_id {
        return Err(NetError::Stun("transaction ID mismatch".into()));
    }

    // Parse attributes to find XOR-MAPPED-ADDRESS or MAPPED-ADDRESS
    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if 20 + msg_len > len {
        return Err(NetError::Stun("message length exceeds packet".into()));
    }

    let mut offset = 20;
    while offset + 4 <= 20 + msg_len {
        let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
        let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > len {
            break;
        }

        if attr_type == STUN_ATTR_XOR_MAPPED_ADDRESS {
            return parse_xor_mapped_address(&buf[offset..offset + attr_len], &txn_id);
        }
        if attr_type == STUN_ATTR_MAPPED_ADDRESS {
            return parse_mapped_address(&buf[offset..offset + attr_len]);
        }

        // Pad to 4-byte boundary
        offset += (attr_len + 3) & !3;
    }

    Err(NetError::Stun("no MAPPED-ADDRESS in response".into()))
}

/// Parse XOR-MAPPED-ADDRESS attribute (RFC 5389 Section 15.2).
fn parse_xor_mapped_address(data: &[u8], _txn_id: &[u8; 12]) -> Result<SocketAddr, NetError> {
    if data.len() < 8 {
        return Err(NetError::Stun("XOR-MAPPED-ADDRESS too short".into()));
    }

    let family = data[1];
    let xored_port = u16::from_be_bytes([data[2], data[3]]);
    let port = xored_port ^ (STUN_MAGIC_COOKIE >> 16) as u16;

    match family {
        0x01 => {
            // IPv4
            let xored_ip = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let ip = xored_ip ^ STUN_MAGIC_COOKIE;
            Ok(SocketAddr::new(
                Ipv4Addr::from(ip.to_be_bytes()).into(),
                port,
            ))
        }
        0x02 => {
            // IPv6
            if data.len() < 20 {
                return Err(NetError::Stun("XOR-MAPPED-ADDRESS IPv6 too short".into()));
            }
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&data[4..20]);
            // XOR with magic cookie + transaction ID
            let cookie_bytes = STUN_MAGIC_COOKIE.to_be_bytes();
            for i in 0..4 {
                ip_bytes[i] ^= cookie_bytes[i];
            }
            for i in 0..12 {
                ip_bytes[4 + i] ^= _txn_id[i];
            }
            Ok(SocketAddr::new(Ipv6Addr::from(ip_bytes).into(), port))
        }
        _ => Err(NetError::Stun(format!("unknown address family: {family}"))),
    }
}

/// Parse MAPPED-ADDRESS attribute (RFC 5389 Section 15.1).
fn parse_mapped_address(data: &[u8]) -> Result<SocketAddr, NetError> {
    if data.len() < 8 {
        return Err(NetError::Stun("MAPPED-ADDRESS too short".into()));
    }

    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            let ip = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Ok(SocketAddr::new(ip.into(), port))
        }
        0x02 => {
            if data.len() < 20 {
                return Err(NetError::Stun("MAPPED-ADDRESS IPv6 too short".into()));
            }
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&data[4..20]);
            Ok(SocketAddr::new(Ipv6Addr::from(ip_bytes).into(), port))
        }
        _ => Err(NetError::Stun(format!("unknown address family: {family}"))),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("bind error: {0}")]
    Bind(String),
    #[error("tls error: {0}")]
    Tls(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("connection closed")]
    ConnectionClosed,
    #[error("write error: {0}")]
    Write(String),
    #[error("read error: {0}")]
    Read(String),
    #[error("message too large: {0} bytes")]
    MessageTooLarge(usize),
    #[error("deserialization error: {0}")]
    Deserialize(String),
    #[error("STUN error: {0}")]
    Stun(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;

    fn make_node_id(byte: u8) -> NodeId {
        NodeId::new([byte; 32])
    }

    #[tokio::test]
    async fn loopback_connection() {
        let server = QuicTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        let client = QuicTransport::client().unwrap();

        let server_task = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            conn.close(0u32.into(), b"done");
        });

        let conn = client.connect(server_addr).await.unwrap();
        conn.close(0u32.into(), b"done");

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn message_send_receive_roundtrip() {
        let server = QuicTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        let client = QuicTransport::client().unwrap();

        let server_task: tokio::task::JoinHandle<DhtMessage> = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            let (_, mut recv) = conn.accept_bi().await.unwrap();
            receive_message(&mut recv).await.unwrap()
        });

        let conn = client.connect(server_addr).await.unwrap();
        let (mut send, _) = conn.open_bi().await.unwrap();
        let ping = DhtMessage::Ping {
            sender: make_node_id(0x42),
        };
        send_message(&mut send, &ping).await.unwrap();
        send.finish().unwrap();

        let received: DhtMessage = server_task.await.unwrap();
        if let DhtMessage::Ping { sender } = received {
            assert_eq!(sender, make_node_id(0x42));
        } else {
            panic!("expected Ping message");
        }
    }

    #[tokio::test]
    async fn blob_streaming() {
        let server = QuicTransport::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr().unwrap();

        let client = QuicTransport::client().unwrap();

        // 1MB test blob
        let test_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let expected = test_data.clone();

        let server_task = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            let (_, mut recv) = conn.accept_bi().await.unwrap();
            let data = receive_blob(&mut recv).await.unwrap();
            data
        });

        let conn = client.connect(server_addr).await.unwrap();
        let (mut send, _) = conn.open_bi().await.unwrap();
        stream_blob(&mut send, &test_data).await.unwrap();

        let received = server_task.await.unwrap();
        assert_eq!(received.len(), expected.len());
        assert_eq!(received, expected);
    }

    #[test]
    fn parse_xor_mapped_address_ipv4() {
        // Family=0x01 (IPv4), port=0x1234 XORed with magic cookie high bits,
        // IP=192.168.1.100 XORed with magic cookie
        let port: u16 = 12345;
        let xored_port = port ^ (STUN_MAGIC_COOKIE >> 16) as u16;
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        let ip_u32 = u32::from(ip);
        let xored_ip = ip_u32 ^ STUN_MAGIC_COOKIE;

        let mut data = [0u8; 8];
        data[0] = 0; // reserved
        data[1] = 0x01; // family IPv4
        data[2..4].copy_from_slice(&xored_port.to_be_bytes());
        data[4..8].copy_from_slice(&xored_ip.to_be_bytes());

        let txn_id = [0u8; 12];
        let result = parse_xor_mapped_address(&data, &txn_id).unwrap();
        assert_eq!(result.port(), port);
        assert_eq!(result.ip(), std::net::IpAddr::V4(ip));
    }

    #[test]
    fn parse_mapped_address_ipv4() {
        let mut data = [0u8; 8];
        data[0] = 0; // reserved
        data[1] = 0x01; // family IPv4
        data[2..4].copy_from_slice(&8080u16.to_be_bytes());
        data[4] = 10;
        data[5] = 0;
        data[6] = 0;
        data[7] = 1;

        let result = parse_mapped_address(&data).unwrap();
        assert_eq!(result, "10.0.0.1:8080".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn parse_stun_too_short() {
        let txn_id = [0u8; 12];
        assert!(parse_xor_mapped_address(&[0; 4], &txn_id).is_err());
        assert!(parse_mapped_address(&[0; 4]).is_err());
    }
}
