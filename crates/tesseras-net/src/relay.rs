//! Transparent QUIC relay: forwards raw UDP packets between NATed peers.
//!
//! The relay reads a 16-byte session token prefix, looks up the session,
//! validates the source address, and forwards the rest verbatim.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Session token size in bytes.
pub const SESSION_TOKEN_SIZE: usize = 16;

/// Default idle timeout for relay sessions.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Rate limit for peers with reciprocity (bytes/sec).
pub const RATE_LIMIT_RECIPROCAL: u32 = 256 * 1024; // 256 KB/s

/// Rate limit for peers without reciprocity (bytes/sec).
pub const RATE_LIMIT_BOOTSTRAP: u32 = 64 * 1024; // 64 KB/s

/// Maximum session duration for non-reciprocal peers.
pub const MAX_BOOTSTRAP_DURATION: Duration = Duration::from_secs(600); // 10 minutes

/// A relay session between two peers.
#[derive(Debug, Clone)]
pub struct RelaySession {
    pub token: [u8; SESSION_TOKEN_SIZE],
    pub peer_a: SocketAddr,
    pub peer_b: SocketAddr,
    pub peer_a_pubkey: [u8; 32],
    pub peer_b_pubkey: [u8; 32],
    pub reciprocity: bool,
    pub bandwidth_limit_bps: u32,
    pub created_at: Instant,
    pub last_active: Instant,
    pub bytes_a_to_b: u64,
    pub bytes_b_to_a: u64,
}

/// Result of forwarding a packet through the relay.
#[derive(Debug, PartialEq, Eq)]
pub enum ForwardResult {
    /// Forward payload to this address.
    Forward {
        dest: SocketAddr,
        payload_offset: usize,
    },
    /// Session not found for this token.
    UnknownSession,
    /// Source address doesn't match either peer.
    UnauthorizedSource,
    /// Rate limit exceeded.
    RateLimitExceeded,
    /// Packet too short to contain session token.
    TooShort,
}

/// Why a session was removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveReason {
    PeerClosed,
    IdleTimeout,
    BootstrapExpired,
}

/// Manages active relay sessions.
pub struct RelaySessionManager {
    sessions: Mutex<HashMap<[u8; SESSION_TOKEN_SIZE], RelaySession>>,
    idle_timeout: Duration,
}

impl RelaySessionManager {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            idle_timeout,
        }
    }

    /// Create a new relay session. Returns the session token.
    pub fn create_session(
        &self,
        peer_a: SocketAddr,
        peer_b: SocketAddr,
        peer_a_pubkey: [u8; 32],
        peer_b_pubkey: [u8; 32],
        reciprocity: bool,
    ) -> [u8; SESSION_TOKEN_SIZE] {
        let token: [u8; SESSION_TOKEN_SIZE] = rand::random();
        let now = Instant::now();
        let session = RelaySession {
            token,
            peer_a,
            peer_b,
            peer_a_pubkey,
            peer_b_pubkey,
            reciprocity,
            bandwidth_limit_bps: if reciprocity {
                RATE_LIMIT_RECIPROCAL
            } else {
                RATE_LIMIT_BOOTSTRAP
            },
            created_at: now,
            last_active: now,
            bytes_a_to_b: 0,
            bytes_b_to_a: 0,
        };
        self.sessions.lock().unwrap().insert(token, session);
        token
    }

    /// Process an incoming packet. Returns where to forward it (if valid).
    /// Packet format: [session_token: 16 bytes][QUIC payload]
    pub fn forward(&self, data: &[u8], from: SocketAddr) -> ForwardResult {
        if data.len() < SESSION_TOKEN_SIZE {
            return ForwardResult::TooShort;
        }

        let token: [u8; SESSION_TOKEN_SIZE] = data[..SESSION_TOKEN_SIZE].try_into().unwrap();

        let mut sessions = self.sessions.lock().unwrap();
        let session = match sessions.get_mut(&token) {
            Some(s) => s,
            None => return ForwardResult::UnknownSession,
        };

        // Determine direction and validate source
        let dest = if from == session.peer_a {
            session.bytes_a_to_b += (data.len() - SESSION_TOKEN_SIZE) as u64;
            session.peer_b
        } else if from == session.peer_b {
            session.bytes_b_to_a += (data.len() - SESSION_TOKEN_SIZE) as u64;
            session.peer_a
        } else {
            return ForwardResult::UnauthorizedSource;
        };

        // Check rate limit (simple: total bytes / elapsed seconds)
        let elapsed = session.created_at.elapsed().as_secs().max(1);
        let total_bytes = session.bytes_a_to_b + session.bytes_b_to_a;
        let avg_bps = total_bytes / elapsed;
        if avg_bps > session.bandwidth_limit_bps as u64 {
            return ForwardResult::RateLimitExceeded;
        }

        session.last_active = Instant::now();

        ForwardResult::Forward {
            dest,
            payload_offset: SESSION_TOKEN_SIZE,
        }
    }

    /// Migrate a session's peer address (after network change).
    /// Returns `true` if migration succeeded.
    pub fn migrate_peer(
        &self,
        token: &[u8; SESSION_TOKEN_SIZE],
        old_addr: SocketAddr,
        new_addr: SocketAddr,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(token) {
            if session.peer_a == old_addr {
                session.peer_a = new_addr;
                session.last_active = Instant::now();
                return true;
            }
            if session.peer_b == old_addr {
                session.peer_b = new_addr;
                session.last_active = Instant::now();
                return true;
            }
        }
        false
    }

    /// Remove a session. Returns it if it existed.
    pub fn remove_session(&self, token: &[u8; SESSION_TOKEN_SIZE]) -> Option<RelaySession> {
        self.sessions.lock().unwrap().remove(token)
    }

    /// Remove expired sessions (idle or bootstrap duration exceeded).
    /// Returns list of (token, reason) for each removed session.
    pub fn cleanup_expired(&self) -> Vec<([u8; SESSION_TOKEN_SIZE], RemoveReason)> {
        let mut sessions = self.sessions.lock().unwrap();
        let mut removed = Vec::new();

        sessions.retain(|token, session| {
            if session.last_active.elapsed() > self.idle_timeout {
                removed.push((*token, RemoveReason::IdleTimeout));
                return false;
            }
            if !session.reciprocity && session.created_at.elapsed() > MAX_BOOTSTRAP_DURATION {
                removed.push((*token, RemoveReason::BootstrapExpired));
                return false;
            }
            true
        });

        removed
    }

    /// Get current session count.
    pub fn active_sessions(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    /// Get a session's public key for a given peer address.
    /// Used by RelayMigrate verification.
    pub fn get_peer_pubkey(
        &self,
        token: &[u8; SESSION_TOKEN_SIZE],
        peer_addr: SocketAddr,
    ) -> Option<[u8; 32]> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(token).and_then(|s| {
            if s.peer_a == peer_addr {
                Some(s.peer_a_pubkey)
            } else if s.peer_b == peer_addr {
                Some(s.peer_b_pubkey)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    #[test]
    fn test_create_and_forward() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let a = addr("10.0.0.1:4433");
        let b = addr("10.0.0.2:4433");

        let token = mgr.create_session(a, b, [1; 32], [2; 32], true);
        assert_eq!(mgr.active_sessions(), 1);

        // A sends to B
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&token);
        pkt.extend_from_slice(b"hello");

        let result = mgr.forward(&pkt, a);
        assert_eq!(
            result,
            ForwardResult::Forward {
                dest: b,
                payload_offset: SESSION_TOKEN_SIZE,
            }
        );

        // B sends to A
        let result = mgr.forward(&pkt, b);
        assert_eq!(
            result,
            ForwardResult::Forward {
                dest: a,
                payload_offset: SESSION_TOKEN_SIZE,
            }
        );
    }

    #[test]
    fn test_unauthorized_source() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let a = addr("10.0.0.1:4433");
        let b = addr("10.0.0.2:4433");
        let c = addr("10.0.0.3:9999");

        let token = mgr.create_session(a, b, [1; 32], [2; 32], true);
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&token);
        pkt.extend_from_slice(b"hello");

        assert_eq!(mgr.forward(&pkt, c), ForwardResult::UnauthorizedSource);
    }

    #[test]
    fn test_unknown_session() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let pkt = vec![0u8; SESSION_TOKEN_SIZE + 5];
        assert_eq!(
            mgr.forward(&pkt, addr("10.0.0.1:4433")),
            ForwardResult::UnknownSession
        );
    }

    #[test]
    fn test_too_short() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        assert_eq!(
            mgr.forward(&[0u8; 10], addr("10.0.0.1:4433")),
            ForwardResult::TooShort
        );
    }

    #[test]
    fn test_migrate_peer() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let a = addr("10.0.0.1:4433");
        let b = addr("10.0.0.2:4433");
        let a_new = addr("10.0.0.1:5555");

        let token = mgr.create_session(a, b, [1; 32], [2; 32], true);

        assert!(mgr.migrate_peer(&token, a, a_new));

        // Now forward should work from new address
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&token);
        pkt.extend_from_slice(b"hello");

        assert_eq!(
            mgr.forward(&pkt, a_new),
            ForwardResult::Forward {
                dest: b,
                payload_offset: SESSION_TOKEN_SIZE,
            }
        );

        // Old address should be rejected
        assert_eq!(mgr.forward(&pkt, a), ForwardResult::UnauthorizedSource);
    }

    #[test]
    fn test_remove_session() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let a = addr("10.0.0.1:4433");
        let b = addr("10.0.0.2:4433");

        let token = mgr.create_session(a, b, [1; 32], [2; 32], true);
        assert_eq!(mgr.active_sessions(), 1);

        let removed = mgr.remove_session(&token);
        assert!(removed.is_some());
        assert_eq!(mgr.active_sessions(), 0);
    }

    #[test]
    fn test_cleanup_idle() {
        let mgr = RelaySessionManager::new(Duration::from_millis(10));
        let a = addr("10.0.0.1:4433");
        let b = addr("10.0.0.2:4433");

        let _token = mgr.create_session(a, b, [1; 32], [2; 32], true);
        assert_eq!(mgr.active_sessions(), 1);

        std::thread::sleep(Duration::from_millis(20));

        let removed = mgr.cleanup_expired();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].1, RemoveReason::IdleTimeout);
        assert_eq!(mgr.active_sessions(), 0);
    }

    #[test]
    fn test_session_tokens_unique() {
        let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
        let mut tokens = std::collections::HashSet::new();
        for _ in 0..100 {
            let token = mgr.create_session(
                addr("10.0.0.1:4433"),
                addr("10.0.0.2:4433"),
                [1; 32],
                [2; 32],
                true,
            );
            assert!(tokens.insert(token), "duplicate token generated");
        }
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn session_tokens_always_unique_pairwise(
                port_a in 1024u16..60000,
                port_b in 1024u16..60000,
            ) {
                let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
                let a = addr(&format!("10.0.0.1:{port_a}"));
                let b = addr(&format!("10.0.0.2:{port_b}"));
                let t1 = mgr.create_session(a, b, [1; 32], [2; 32], true);
                let t2 = mgr.create_session(a, b, [1; 32], [2; 32], true);
                prop_assert_ne!(t1, t2);
            }

            #[test]
            fn forward_too_short_always_rejected(
                data_len in 0usize..16,
            ) {
                let mgr = RelaySessionManager::new(DEFAULT_IDLE_TIMEOUT);
                let data = vec![0u8; data_len];
                let result = mgr.forward(&data, addr("10.0.0.1:4433"));
                prop_assert_eq!(result, ForwardResult::TooShort);
            }
        }
    }
}
