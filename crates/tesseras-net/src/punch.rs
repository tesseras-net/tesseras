//! Hole-punch coordination: signed introductions and retry logic.

use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use tesseras_core::NodeId;

/// Maximum age of a PunchIntro timestamp (seconds).
const TIMESTAMP_WINDOW_SECS: u64 = 30;

/// Build the payload to sign for a PunchIntro: target || external_addr || timestamp.
fn punch_intro_payload(target: &NodeId, external_addr: &SocketAddr, timestamp: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(20 + 18 + 8);
    buf.extend_from_slice(target.as_bytes());
    buf.extend_from_slice(external_addr.to_string().as_bytes());
    buf.extend_from_slice(&timestamp.to_be_bytes());
    buf
}

/// Sign a PunchIntro.
pub fn sign_punch_intro(
    target: &NodeId,
    external_addr: &SocketAddr,
    timestamp: u64,
    signing_key: &SigningKey,
) -> [u8; 64] {
    let payload = punch_intro_payload(target, external_addr, timestamp);
    let sig: Signature = signing_key.sign(&payload);
    sig.to_bytes()
}

/// Verify a PunchIntro signature. Returns `Ok(())` if valid.
pub fn verify_punch_intro(
    target: &NodeId,
    external_addr: &SocketAddr,
    timestamp: u64,
    signature: &[u8; 64],
    verifying_key: &VerifyingKey,
) -> Result<(), PunchError> {
    // Check timestamp freshness
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if now.saturating_sub(timestamp) > TIMESTAMP_WINDOW_SECS {
        return Err(PunchError::TimestampExpired {
            age_secs: now - timestamp,
        });
    }
    if timestamp > now + 5 {
        return Err(PunchError::TimestampInFuture);
    }

    let payload = punch_intro_payload(target, external_addr, timestamp);
    let sig = Signature::from_bytes(signature);
    verifying_key
        .verify(&payload, &sig)
        .map_err(|_| PunchError::InvalidSignature)
}

/// Build the payload to sign for a RelayMigrate: session_token || timestamp.
fn relay_migrate_payload(session_token: &[u8; 16], timestamp: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(24);
    buf.extend_from_slice(session_token);
    buf.extend_from_slice(&timestamp.to_be_bytes());
    buf
}

/// Sign a RelayMigrate message.
pub fn sign_relay_migrate(
    session_token: &[u8; 16],
    timestamp: u64,
    signing_key: &SigningKey,
) -> [u8; 64] {
    let payload = relay_migrate_payload(session_token, timestamp);
    let sig: Signature = signing_key.sign(&payload);
    sig.to_bytes()
}

/// Verify a RelayMigrate signature.
pub fn verify_relay_migrate(
    session_token: &[u8; 16],
    timestamp: u64,
    signature: &[u8; 64],
    verifying_key: &VerifyingKey,
) -> Result<(), PunchError> {
    let payload = relay_migrate_payload(session_token, timestamp);
    let sig = Signature::from_bytes(signature);
    verifying_key
        .verify(&payload, &sig)
        .map_err(|_| PunchError::InvalidSignature)
}

/// Build the payload to sign for a RelayRequest: target || timestamp.
fn relay_request_payload(target: &NodeId, timestamp: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(28);
    buf.extend_from_slice(target.as_bytes());
    buf.extend_from_slice(&timestamp.to_be_bytes());
    buf
}

/// Sign a RelayRequest message.
pub fn sign_relay_request(target: &NodeId, timestamp: u64, signing_key: &SigningKey) -> [u8; 64] {
    let payload = relay_request_payload(target, timestamp);
    let sig: Signature = signing_key.sign(&payload);
    sig.to_bytes()
}

/// Verify a RelayRequest signature.
pub fn verify_relay_request(
    target: &NodeId,
    timestamp: u64,
    signature: &[u8; 64],
    verifying_key: &VerifyingKey,
) -> Result<(), PunchError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if now.saturating_sub(timestamp) > TIMESTAMP_WINDOW_SECS {
        return Err(PunchError::TimestampExpired {
            age_secs: now - timestamp,
        });
    }

    let payload = relay_request_payload(target, timestamp);
    let sig = Signature::from_bytes(signature);
    verifying_key
        .verify(&payload, &sig)
        .map_err(|_| PunchError::InvalidSignature)
}

#[derive(Debug, thiserror::Error)]
pub enum PunchError {
    #[error("timestamp expired ({age_secs}s old, max {TIMESTAMP_WINDOW_SECS}s)")]
    TimestampExpired { age_secs: u64 },
    #[error("timestamp is in the future")]
    TimestampInFuture,
    #[error("invalid Ed25519 signature")]
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn test_punch_intro_sign_verify() {
        let key = SigningKey::generate(&mut OsRng);
        let target = NodeId::new([3u8; 20]);
        let addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        let ts = now_secs();

        let sig = sign_punch_intro(&target, &addr, ts, &key);
        assert!(verify_punch_intro(&target, &addr, ts, &sig, &key.verifying_key()).is_ok());
    }

    #[test]
    fn test_punch_intro_wrong_key() {
        let key = SigningKey::generate(&mut OsRng);
        let wrong_key = SigningKey::generate(&mut OsRng);
        let target = NodeId::new([3u8; 20]);
        let addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        let ts = now_secs();

        let sig = sign_punch_intro(&target, &addr, ts, &key);
        let result = verify_punch_intro(&target, &addr, ts, &sig, &wrong_key.verifying_key());
        assert!(matches!(result, Err(PunchError::InvalidSignature)));
    }

    #[test]
    fn test_punch_intro_tampered_addr() {
        let key = SigningKey::generate(&mut OsRng);
        let target = NodeId::new([3u8; 20]);
        let addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        let fake_addr: SocketAddr = "10.0.0.1:4433".parse().unwrap();
        let ts = now_secs();

        let sig = sign_punch_intro(&target, &addr, ts, &key);
        // Verify with tampered address
        let result = verify_punch_intro(&target, &fake_addr, ts, &sig, &key.verifying_key());
        assert!(matches!(result, Err(PunchError::InvalidSignature)));
    }

    #[test]
    fn test_punch_intro_expired() {
        let key = SigningKey::generate(&mut OsRng);
        let target = NodeId::new([3u8; 20]);
        let addr: SocketAddr = "203.0.113.5:4433".parse().unwrap();
        let ts = now_secs() - 60; // 60 seconds ago

        let sig = sign_punch_intro(&target, &addr, ts, &key);
        let result = verify_punch_intro(&target, &addr, ts, &sig, &key.verifying_key());
        assert!(matches!(result, Err(PunchError::TimestampExpired { .. })));
    }

    #[test]
    fn test_relay_migrate_sign_verify() {
        let key = SigningKey::generate(&mut OsRng);
        let token = [0xAA; 16];
        let ts = now_secs();

        let sig = sign_relay_migrate(&token, ts, &key);
        assert!(verify_relay_migrate(&token, ts, &sig, &key.verifying_key()).is_ok());
    }

    #[test]
    fn test_relay_request_sign_verify() {
        let key = SigningKey::generate(&mut OsRng);
        let target = NodeId::new([5u8; 20]);
        let ts = now_secs();

        let sig = sign_relay_request(&target, ts, &key);
        assert!(verify_relay_request(&target, ts, &sig, &key.verifying_key()).is_ok());
    }
}
