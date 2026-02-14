use serde::{Deserialize, Serialize};

/// Maximum wire message size: 64 KiB.
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Application-level QUIC error codes.
pub const ERR_MESSAGE_TOO_LARGE: u32 = 0x01;
pub const ERR_INVALID_MESSAGE: u32 = 0x02;
pub const ERR_UNSUPPORTED: u32 = 0x03;
pub const ERR_OVERLOADED: u32 = 0x04;

/// ALPN protocol identifier for tesseras/1.
pub const ALPN_TESSERAS_V1: &[u8] = b"tesseras/1";

/// Top-level wire message with version and request_id for logging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WireMessage {
    pub version: u8,
    pub request_id: u64,
    pub body: WireBody,
}

/// Wire body: request, response, or protocol-level error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WireBody {
    Request(Vec<u8>),
    Response(Vec<u8>),
    Error { code: u16, reason: String },
}

use crate::error::NetError;

/// Encode a WireMessage into length-prefixed bytes.
/// Format: [4-byte big-endian u32 length][msgpack payload]
pub fn encode(msg: &WireMessage) -> Result<Vec<u8>, NetError> {
    let payload =
        rmp_serde::to_vec(msg).map_err(|e| NetError::InvalidMessage(e.to_string()))?;
    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(NetError::MessageTooLarge {
            size: payload.len(),
            max: MAX_MESSAGE_SIZE,
        });
    }
    let len = payload.len() as u32;
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Decode a length-prefixed WireMessage from bytes.
/// Returns (WireMessage, bytes_consumed).
pub fn decode(data: &[u8]) -> Result<(WireMessage, usize), NetError> {
    if data.len() < 4 {
        return Err(NetError::InvalidMessage(
            "too short for length prefix".into(),
        ));
    }
    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(NetError::MessageTooLarge {
            size: len,
            max: MAX_MESSAGE_SIZE,
        });
    }
    if data.len() < 4 + len {
        return Err(NetError::InvalidMessage("incomplete message".into()));
    }
    let msg: WireMessage = rmp_serde::from_slice(&data[4..4 + len])
        .map_err(|e| NetError::InvalidMessage(e.to_string()))?;
    Ok((msg, 4 + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: u64, payload: &[u8]) -> WireMessage {
        WireMessage {
            version: 1,
            request_id: id,
            body: WireBody::Request(payload.to_vec()),
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let msg = make_request(42, b"hello");
        let encoded = encode(&msg).unwrap();
        let (decoded, consumed) = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn encode_decode_error_body() {
        let msg = WireMessage {
            version: 1,
            request_id: 1,
            body: WireBody::Error {
                code: ERR_OVERLOADED as u16,
                reason: "too busy".into(),
            },
        };
        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn encode_rejects_oversized_message() {
        let big_payload = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let msg = make_request(1, &big_payload);
        let result = encode(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn decode_rejects_short_input() {
        let result = decode(&[0, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_rejects_incomplete_input() {
        let msg = make_request(1, b"test");
        let encoded = encode(&msg).unwrap();
        let result = decode(&encoded[..encoded.len() - 2]);
        assert!(result.is_err());
    }

    #[test]
    fn length_prefix_is_big_endian() {
        let msg = make_request(1, b"x");
        let encoded = encode(&msg).unwrap();
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(len as usize, encoded.len() - 4);
    }
}
