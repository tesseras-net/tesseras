//! Mobile reconnection state machine.
//!
//! Three phases after a network change:
//! 1. QUIC connection migration (0-2s)
//! 2. Re-STUN + DHT re-announce (2-5s)
//! 3. Re-establish failed peers (5-10s)

use std::net::SocketAddr;
use std::time::Duration;

use tesseras_core::network::NatType;

/// Reconnection phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectPhase {
    /// Try QUIC connection migration for all active peers.
    QuicMigration,
    /// Re-discover external address via STUN.
    ReStun,
    /// Re-establish connections that migration couldn't save.
    ReEstablish,
    /// All phases complete.
    Done,
}

/// Priority tier for peer reconnection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReconnectPriority {
    /// Bootstrap / public-IP DHT neighbors (needed for everything else).
    Bootstrap = 0,
    /// Peers storing our tessera fragments.
    OurFragmentHolders = 1,
    /// Peers whose fragments we store (reciprocity).
    TheirFragmentHolders = 2,
    /// General DHT neighbors.
    DhtNeighbor = 3,
}

/// Configuration for reconnection timeouts.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Timeout for Phase 1 QUIC migration per peer.
    pub migration_timeout: Duration,
    /// Timeout for Phase 2 STUN re-discovery.
    pub stun_timeout: Duration,
    /// Maximum total reconnection time.
    pub total_timeout: Duration,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            migration_timeout: Duration::from_secs(2),
            stun_timeout: Duration::from_secs(3),
            total_timeout: Duration::from_secs(10),
        }
    }
}

/// Peer info for reconnection prioritization.
#[derive(Debug, Clone)]
pub struct ReconnectPeer {
    pub addr: SocketAddr,
    pub node_id: [u8; 20],
    pub nat_type: NatType,
    pub priority: ReconnectPriority,
    pub migrated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_priority_ordering() {
        assert!(ReconnectPriority::Bootstrap < ReconnectPriority::OurFragmentHolders);
        assert!(ReconnectPriority::OurFragmentHolders < ReconnectPriority::TheirFragmentHolders);
        assert!(ReconnectPriority::TheirFragmentHolders < ReconnectPriority::DhtNeighbor);
    }

    #[test]
    fn reconnect_config_defaults() {
        let config = ReconnectConfig::default();
        assert_eq!(config.migration_timeout, Duration::from_secs(2));
        assert_eq!(config.stun_timeout, Duration::from_secs(3));
        assert_eq!(config.total_timeout, Duration::from_secs(10));
    }

    #[test]
    fn reconnect_phase_enum() {
        // Verify all phases exist and are distinct
        let phases = [
            ReconnectPhase::QuicMigration,
            ReconnectPhase::ReStun,
            ReconnectPhase::ReEstablish,
            ReconnectPhase::Done,
        ];
        for (i, p1) in phases.iter().enumerate() {
            for (j, p2) in phases.iter().enumerate() {
                if i == j {
                    assert_eq!(p1, p2);
                } else {
                    assert_ne!(p1, p2);
                }
            }
        }
    }

    #[test]
    fn reconnect_peer_priority_sort() {
        let peers = vec![
            ReconnectPeer {
                addr: "10.0.0.1:4433".parse().unwrap(),
                node_id: [1; 20],
                nat_type: NatType::Cone,
                priority: ReconnectPriority::DhtNeighbor,
                migrated: false,
            },
            ReconnectPeer {
                addr: "10.0.0.2:4433".parse().unwrap(),
                node_id: [2; 20],
                nat_type: NatType::Public,
                priority: ReconnectPriority::Bootstrap,
                migrated: false,
            },
            ReconnectPeer {
                addr: "10.0.0.3:4433".parse().unwrap(),
                node_id: [3; 20],
                nat_type: NatType::Cone,
                priority: ReconnectPriority::OurFragmentHolders,
                migrated: false,
            },
        ];

        let mut sorted = peers.clone();
        sorted.sort_by_key(|p| p.priority);

        assert_eq!(sorted[0].priority, ReconnectPriority::Bootstrap);
        assert_eq!(sorted[1].priority, ReconnectPriority::OurFragmentHolders);
        assert_eq!(sorted[2].priority, ReconnectPriority::DhtNeighbor);
    }
}
