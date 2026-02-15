//! Minimal STUN client for NAT type detection (RFC 5389).
//!
//! Only implements Binding Request/Response — enough to discover the node's
//! external address from public STUN servers. Shares quinn's UDP socket.

use std::net::SocketAddr;

use tesseras_core::network::NatType;

/// STUN magic cookie (RFC 5389 Section 6).
pub const MAGIC_COOKIE: u32 = 0x2112_A442;

/// STUN message type: Binding Request.
const BINDING_REQUEST: u16 = 0x0001;

/// STUN message type: Binding Success Response.
const BINDING_RESPONSE: u16 = 0x0101;

/// STUN attribute type: XOR-MAPPED-ADDRESS.
const XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// STUN attribute type: MAPPED-ADDRESS (fallback).
const MAPPED_ADDRESS: u16 = 0x0001;

/// STUN header size in bytes.
const HEADER_SIZE: usize = 20;

/// Check if a UDP packet is a STUN message by inspecting the magic cookie.
/// Bytes 4-7 must match `0x2112A442`.
pub fn is_stun_packet(data: &[u8]) -> bool {
    data.len() >= HEADER_SIZE
        && u32::from_be_bytes([data[4], data[5], data[6], data[7]]) == MAGIC_COOKIE
}

/// Encode a STUN Binding Request.
/// Returns 20 bytes: 2 (type) + 2 (length=0) + 4 (cookie) + 12 (transaction ID).
pub fn encode_binding_request(transaction_id: &[u8; 12]) -> [u8; HEADER_SIZE] {
    let mut buf = [0u8; HEADER_SIZE];
    // Message type: Binding Request
    buf[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    // Message length: 0 (no attributes)
    buf[2..4].copy_from_slice(&0u16.to_be_bytes());
    // Magic cookie
    buf[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    // Transaction ID
    buf[8..20].copy_from_slice(transaction_id);
    buf
}

/// Decoded STUN Binding Response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingResponse {
    pub transaction_id: [u8; 12],
    pub mapped_addr: SocketAddr,
}

/// Decode a STUN Binding Success Response, extracting XOR-MAPPED-ADDRESS.
/// Falls back to MAPPED-ADDRESS if XOR variant is absent.
pub fn decode_binding_response(data: &[u8]) -> Result<BindingResponse, StunError> {
    if data.len() < HEADER_SIZE {
        return Err(StunError::TooShort);
    }

    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != BINDING_RESPONSE {
        return Err(StunError::NotBindingResponse { msg_type });
    }

    let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if cookie != MAGIC_COOKIE {
        return Err(StunError::BadMagicCookie);
    }

    let mut transaction_id = [0u8; 12];
    transaction_id.copy_from_slice(&data[8..20]);

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    if data.len() < HEADER_SIZE + msg_len {
        return Err(StunError::TooShort);
    }

    let attrs = &data[HEADER_SIZE..HEADER_SIZE + msg_len];
    let mut mapped = None;

    let mut offset = 0;
    while offset + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[offset], attrs[offset + 1]]);
        let attr_len = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) as usize;
        let attr_data = &attrs[offset + 4..offset + 4 + attr_len];

        match attr_type {
            XOR_MAPPED_ADDRESS => {
                mapped = Some(decode_xor_mapped_address(attr_data, &transaction_id)?);
            }
            MAPPED_ADDRESS if mapped.is_none() => {
                mapped = Some(decode_mapped_address(attr_data)?);
            }
            _ => {}
        }

        // Attributes are padded to 4-byte boundaries
        let padded_len = (attr_len + 3) & !3;
        offset += 4 + padded_len;
    }

    mapped
        .map(|addr| BindingResponse {
            transaction_id,
            mapped_addr: addr,
        })
        .ok_or(StunError::NoMappedAddress)
}

fn decode_xor_mapped_address(
    data: &[u8],
    transaction_id: &[u8; 12],
) -> Result<SocketAddr, StunError> {
    if data.len() < 4 {
        return Err(StunError::TooShort);
    }
    let family = data[1];
    let xor_port = u16::from_be_bytes([data[2], data[3]]);
    let port = xor_port ^ (MAGIC_COOKIE >> 16) as u16;

    match family {
        0x01 => {
            // IPv4
            if data.len() < 8 {
                return Err(StunError::TooShort);
            }
            let xor_ip = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let ip = xor_ip ^ MAGIC_COOKIE;
            let addr = std::net::Ipv4Addr::from(ip);
            Ok(SocketAddr::new(addr.into(), port))
        }
        0x02 => {
            // IPv6
            if data.len() < 20 {
                return Err(StunError::TooShort);
            }
            let mut xor_ip = [0u8; 16];
            xor_ip.copy_from_slice(&data[4..20]);
            // XOR with magic cookie (4 bytes) + transaction ID (12 bytes)
            let mut mask = [0u8; 16];
            mask[0..4].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
            mask[4..16].copy_from_slice(transaction_id);
            for i in 0..16 {
                xor_ip[i] ^= mask[i];
            }
            let addr = std::net::Ipv6Addr::from(xor_ip);
            Ok(SocketAddr::new(addr.into(), port))
        }
        _ => Err(StunError::UnknownAddressFamily { family }),
    }
}

fn decode_mapped_address(data: &[u8]) -> Result<SocketAddr, StunError> {
    if data.len() < 4 {
        return Err(StunError::TooShort);
    }
    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            if data.len() < 8 {
                return Err(StunError::TooShort);
            }
            let ip = std::net::Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Ok(SocketAddr::new(ip.into(), port))
        }
        0x02 => {
            if data.len() < 20 {
                return Err(StunError::TooShort);
            }
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&data[4..20]);
            let ip = std::net::Ipv6Addr::from(ip_bytes);
            Ok(SocketAddr::new(ip.into(), port))
        }
        _ => Err(StunError::UnknownAddressFamily { family }),
    }
}

/// Classify NAT type from STUN discovery results.
///
/// Takes a list of `(stun_server_addr, mapped_addr)` pairs and the local bind address.
/// - If any mapped_addr equals local_addr → `Public`
/// - If all mapped_addrs are identical → `Cone` (consistent mapping)
/// - If mapped_addrs differ → `Symmetric` (per-destination mapping)
/// - If list is empty → `Unknown`
pub fn classify_nat(local_addr: SocketAddr, results: &[(SocketAddr, SocketAddr)]) -> NatType {
    if results.is_empty() {
        return NatType::Unknown;
    }

    // Check if external matches local (public IP)
    let first_mapped = results[0].1;
    if first_mapped.ip() == local_addr.ip() {
        return NatType::Public;
    }

    // Check if all mapped addresses are the same
    let all_same = results.iter().all(|(_, mapped)| *mapped == first_mapped);
    if all_same {
        NatType::Cone
    } else {
        NatType::Symmetric
    }
}

/// STUN decode errors.
#[derive(Debug, thiserror::Error)]
pub enum StunError {
    #[error("packet too short")]
    TooShort,
    #[error("not a binding response (type=0x{msg_type:04x})")]
    NotBindingResponse { msg_type: u16 },
    #[error("bad magic cookie")]
    BadMagicCookie,
    #[error("no MAPPED-ADDRESS or XOR-MAPPED-ADDRESS attribute")]
    NoMappedAddress,
    #[error("unknown address family: {family}")]
    UnknownAddressFamily { family: u8 },
    #[error("all {attempts} retries failed for {server}")]
    AllRetriesFailed { server: SocketAddr, attempts: u8 },
}

use std::time::Duration;
use tokio::net::UdpSocket;

/// Configuration for STUN discovery.
#[derive(Debug, Clone)]
pub struct StunConfig {
    /// STUN server addresses to query.
    pub servers: Vec<SocketAddr>,
    /// Timeout per STUN request attempt.
    pub timeout: Duration,
    /// Number of retry attempts per server.
    pub retries: u8,
}

impl Default for StunConfig {
    fn default() -> Self {
        Self {
            servers: vec![
                // Google STUN
                "74.125.250.129:19302".parse().unwrap(),
                // Cloudflare STUN
                "162.159.200.1:3478".parse().unwrap(),
            ],
            timeout: Duration::from_millis(500),
            retries: 3,
        }
    }
}

/// Discover external address by querying a single STUN server.
/// `socket` should be the same UDP socket quinn is bound to.
///
/// Note: In production, this is called via the packet filter on quinn's socket.
/// For unit testing, we use a standalone UDP socket.
pub async fn stun_query(
    socket: &UdpSocket,
    server: SocketAddr,
    timeout: Duration,
    retries: u8,
) -> Result<BindingResponse, StunError> {
    let transaction_id: [u8; 12] = rand::random();
    let request = encode_binding_request(&transaction_id);

    for attempt in 0..retries {
        if let Err(e) = socket.send_to(&request, server).await {
            tracing::debug!(
                attempt,
                server = %server,
                error = %e,
                "STUN send failed"
            );
            continue;
        }

        let mut buf = [0u8; 576]; // RFC 5389 minimum MTU
        match tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, from))) => {
                if from != server {
                    tracing::debug!(
                        expected = %server,
                        got = %from,
                        "STUN response from unexpected source"
                    );
                    continue;
                }
                match decode_binding_response(&buf[..len]) {
                    Ok(resp) if resp.transaction_id == transaction_id => return Ok(resp),
                    Ok(_) => {
                        tracing::debug!("STUN transaction ID mismatch");
                        continue;
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "STUN decode failed");
                        continue;
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::debug!(
                    attempt,
                    server = %server,
                    error = %e,
                    "STUN recv failed"
                );
            }
            Err(_) => {
                tracing::debug!(
                    attempt,
                    server = %server,
                    "STUN request timed out"
                );
            }
        }
    }

    Err(StunError::AllRetriesFailed {
        server,
        attempts: retries,
    })
}

/// Discover NAT type by querying multiple STUN servers.
/// Returns `(NatType, Option<SocketAddr>)` — the classified NAT type
/// and the external address (if discovered).
pub async fn discover_nat(
    socket: &UdpSocket,
    config: &StunConfig,
) -> (NatType, Option<SocketAddr>) {
    let local_addr = match socket.local_addr() {
        Ok(addr) => addr,
        Err(_) => return (NatType::Unknown, None),
    };

    let mut results = Vec::new();
    for server in &config.servers {
        match stun_query(socket, *server, config.timeout, config.retries).await {
            Ok(resp) => {
                tracing::info!(
                    server = %server,
                    mapped = %resp.mapped_addr,
                    "STUN binding response"
                );
                results.push((*server, resp.mapped_addr));
            }
            Err(e) => {
                tracing::warn!(server = %server, error = %e, "STUN query failed");
            }
        }
    }

    let nat_type = classify_nat(local_addr, &results);
    let external_addr = results.first().map(|(_, addr)| *addr);

    tracing::info!(nat_type = %nat_type, external_addr = ?external_addr, "NAT detection complete");

    (nat_type, external_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_stun_packet() {
        let txn = [1u8; 12];
        let pkt = encode_binding_request(&txn);
        assert!(is_stun_packet(&pkt));
        assert!(!is_stun_packet(&[0u8; 20]));
        assert!(!is_stun_packet(&[0u8; 10]));
    }

    #[test]
    fn test_encode_binding_request() {
        let txn = [0xAA; 12];
        let pkt = encode_binding_request(&txn);
        assert_eq!(pkt.len(), 20);
        // Type: Binding Request
        assert_eq!(u16::from_be_bytes([pkt[0], pkt[1]]), 0x0001);
        // Length: 0
        assert_eq!(u16::from_be_bytes([pkt[2], pkt[3]]), 0);
        // Magic cookie
        assert_eq!(
            u32::from_be_bytes([pkt[4], pkt[5], pkt[6], pkt[7]]),
            MAGIC_COOKIE
        );
        // Transaction ID
        assert_eq!(&pkt[8..20], &[0xAA; 12]);
    }

    #[test]
    fn test_decode_binding_response_ipv4() {
        // Build a minimal Binding Response with XOR-MAPPED-ADDRESS
        let txn = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let addr: SocketAddr = "203.0.113.5:12345".parse().unwrap();

        let port_xor = (addr.port()) ^ (MAGIC_COOKIE >> 16) as u16;
        let ip_bytes = match addr.ip() {
            std::net::IpAddr::V4(ip) => u32::from(ip),
            _ => unreachable!(),
        };
        let ip_xor = ip_bytes ^ MAGIC_COOKIE;

        // XOR-MAPPED-ADDRESS attribute: family(IPv4), port, ip
        let mut attr = vec![0x00, 0x01]; // reserved + family
        attr.extend_from_slice(&port_xor.to_be_bytes());
        attr.extend_from_slice(&ip_xor.to_be_bytes());

        // Full attribute TLV
        let mut attrs = vec![];
        attrs.extend_from_slice(&XOR_MAPPED_ADDRESS.to_be_bytes()); // type
        attrs.extend_from_slice(&(attr.len() as u16).to_be_bytes()); // length
        attrs.extend_from_slice(&attr);

        // Full STUN message
        let mut msg = vec![];
        msg.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
        msg.extend_from_slice(&(attrs.len() as u16).to_be_bytes());
        msg.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        msg.extend_from_slice(&txn);
        msg.extend_from_slice(&attrs);

        let resp = decode_binding_response(&msg).unwrap();
        assert_eq!(resp.transaction_id, txn);
        assert_eq!(resp.mapped_addr, addr);
    }

    #[test]
    fn test_decode_not_binding_response() {
        let pkt = encode_binding_request(&[0; 12]);
        // It's a request, not a response
        let err = decode_binding_response(&pkt).unwrap_err();
        assert!(matches!(err, StunError::NotBindingResponse { .. }));
    }

    #[test]
    fn test_decode_too_short() {
        let err = decode_binding_response(&[0; 5]).unwrap_err();
        assert!(matches!(err, StunError::TooShort));
    }

    // --- NAT classification tests ---

    #[test]
    fn test_classify_nat_public() {
        let local: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        let stun1: SocketAddr = "198.51.100.1:3478".parse().unwrap();
        let mapped: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        assert_eq!(classify_nat(local, &[(stun1, mapped)]), NatType::Public);
    }

    #[test]
    fn test_classify_nat_cone() {
        let local: SocketAddr = "192.168.1.100:4433".parse().unwrap();
        let stun1: SocketAddr = "198.51.100.1:3478".parse().unwrap();
        let stun2: SocketAddr = "198.51.100.2:3478".parse().unwrap();
        let mapped: SocketAddr = "203.0.113.5:12345".parse().unwrap();
        assert_eq!(
            classify_nat(local, &[(stun1, mapped), (stun2, mapped)]),
            NatType::Cone
        );
    }

    #[test]
    fn test_classify_nat_symmetric() {
        let local: SocketAddr = "192.168.1.100:4433".parse().unwrap();
        let stun1: SocketAddr = "198.51.100.1:3478".parse().unwrap();
        let stun2: SocketAddr = "198.51.100.2:3478".parse().unwrap();
        let mapped1: SocketAddr = "203.0.113.5:12345".parse().unwrap();
        let mapped2: SocketAddr = "203.0.113.5:12346".parse().unwrap();
        assert_eq!(
            classify_nat(local, &[(stun1, mapped1), (stun2, mapped2)]),
            NatType::Symmetric
        );
    }

    #[test]
    fn test_classify_nat_unknown() {
        let local: SocketAddr = "192.168.1.100:4433".parse().unwrap();
        assert_eq!(classify_nat(local, &[]), NatType::Unknown);
    }

    // --- Async STUN discovery tests ---

    #[tokio::test]
    async fn test_stun_query_loopback() {
        // Spin up a fake STUN server on loopback
        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        let client_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client_addr = client_sock.local_addr().unwrap();

        // Fake server: read request, reply with XOR-MAPPED-ADDRESS = client_addr
        let server_handle = tokio::spawn(async move {
            let mut buf = [0u8; 576];
            let (len, from) = server_sock.recv_from(&mut buf).await.unwrap();
            assert!(is_stun_packet(&buf[..len]));

            let txn_id: [u8; 12] = buf[8..20].try_into().unwrap();

            // Build XOR-MAPPED-ADDRESS for client_addr
            let port_xor = from.port() ^ (MAGIC_COOKIE >> 16) as u16;
            let ip_xor = match from.ip() {
                std::net::IpAddr::V4(ip) => u32::from(ip) ^ MAGIC_COOKIE,
                _ => unreachable!(),
            };

            let mut attr = vec![0x00, 0x01]; // reserved + family IPv4
            attr.extend_from_slice(&port_xor.to_be_bytes());
            attr.extend_from_slice(&ip_xor.to_be_bytes());

            let mut attrs = vec![];
            attrs.extend_from_slice(&XOR_MAPPED_ADDRESS.to_be_bytes());
            attrs.extend_from_slice(&(attr.len() as u16).to_be_bytes());
            attrs.extend_from_slice(&attr);

            let mut resp = vec![];
            resp.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
            resp.extend_from_slice(&(attrs.len() as u16).to_be_bytes());
            resp.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
            resp.extend_from_slice(&txn_id);
            resp.extend_from_slice(&attrs);

            server_sock.send_to(&resp, from).await.unwrap();
        });

        let result = stun_query(&client_sock, server_addr, Duration::from_secs(1), 1)
            .await
            .unwrap();

        assert_eq!(result.mapped_addr, client_addr);
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_stun_query_timeout() {
        // Server that never responds
        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();
        // Keep server_sock alive but don't read from it
        let _keep = server_sock;

        let client_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let result = stun_query(&client_sock, server_addr, Duration::from_millis(50), 2).await;

        assert!(matches!(result, Err(StunError::AllRetriesFailed { .. })));
    }

    #[tokio::test]
    async fn test_discover_nat_loopback_is_public() {
        // On loopback, mapped address == local address → Public
        let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_sock.local_addr().unwrap();

        let client_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        // Fake STUN server echoes back the client's address
        tokio::spawn(async move {
            let mut buf = [0u8; 576];
            let (_len, from) = server_sock.recv_from(&mut buf).await.unwrap();
            let txn_id: [u8; 12] = buf[8..20].try_into().unwrap();

            let port_xor = from.port() ^ (MAGIC_COOKIE >> 16) as u16;
            let ip_xor = match from.ip() {
                std::net::IpAddr::V4(ip) => u32::from(ip) ^ MAGIC_COOKIE,
                _ => unreachable!(),
            };

            let mut attr = vec![0x00, 0x01];
            attr.extend_from_slice(&port_xor.to_be_bytes());
            attr.extend_from_slice(&ip_xor.to_be_bytes());

            let mut attrs = vec![];
            attrs.extend_from_slice(&XOR_MAPPED_ADDRESS.to_be_bytes());
            attrs.extend_from_slice(&(attr.len() as u16).to_be_bytes());
            attrs.extend_from_slice(&attr);

            let mut resp = vec![];
            resp.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
            resp.extend_from_slice(&(attrs.len() as u16).to_be_bytes());
            resp.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
            resp.extend_from_slice(&txn_id);
            resp.extend_from_slice(&attrs);

            server_sock.send_to(&resp, from).await.unwrap();
        });

        let config = StunConfig {
            servers: vec![server_addr],
            timeout: Duration::from_secs(1),
            retries: 1,
        };

        let (nat_type, ext_addr) = discover_nat(&client_sock, &config).await;
        assert_eq!(nat_type, NatType::Public);
        assert!(ext_addr.is_some());
    }
}
