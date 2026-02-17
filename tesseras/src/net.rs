use std::net::SocketAddr;
use std::sync::Arc;

use quinn::{Endpoint, ServerConfig};

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
    pub async fn bind(listen_addr: SocketAddr) -> Result<Self, NetError> {
        let server_config = make_server_config()?;
        let endpoint = Endpoint::server(server_config, listen_addr)
            .map_err(|e| NetError::Bind(e.to_string()))?;
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
}
