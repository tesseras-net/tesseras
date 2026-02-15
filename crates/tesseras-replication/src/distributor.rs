use std::collections::HashMap;
use std::net::SocketAddr;

use tesseras_core::{Capabilities, NodeInfo};

/// Extract /24 subnet from a socket address.
fn extract_subnet(addr: &SocketAddr) -> String {
    match addr.ip() {
        std::net::IpAddr::V4(ip) => {
            let octets = ip.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        std::net::IpAddr::V6(ip) => {
            // Use first 48 bits for IPv6 subnet diversity
            let segments = ip.segments();
            format!("{:x}:{:x}:{:x}", segments[0], segments[1], segments[2])
        }
    }
}

/// Filter peers to limit the number from the same /24 subnet.
pub fn apply_subnet_diversity(peers: &[NodeInfo], max_per_subnet: usize) -> Vec<NodeInfo> {
    let mut subnet_counts: HashMap<String, usize> = HashMap::new();
    let mut result = Vec::new();

    for peer in peers {
        let subnet = extract_subnet(&peer.addr);
        let count = subnet_counts.entry(subnet).or_insert(0);
        if *count < max_per_subnet {
            *count += 1;
            result.push(peer.clone());
        }
    }

    result
}

/// Filter peers to limit institutional nodes to max ceil(r / 3.5) per holder set.
///
/// This prevents concentration risk: if 3 major institutions leave simultaneously,
/// no fragment loses more than 2 of its 7 holders.
pub fn apply_institutional_diversity(peers: &[NodeInfo], target_r: usize) -> Vec<NodeInfo> {
    let max_institutional = (target_r as f64 / 3.5).ceil() as usize;

    let mut institutional_count = 0;
    let mut result = Vec::new();

    // First pass: add peers respecting institutional limit
    for peer in peers {
        if peer.capabilities.has(Capabilities::INSTITUTIONAL) {
            if institutional_count < max_institutional {
                institutional_count += 1;
                result.push(peer.clone());
            }
        } else {
            result.push(peer.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tesseras_core::types::NodeId;
    use tesseras_core::{Capabilities, NodeIdentity};

    fn node(fill: u8) -> NodeId {
        NodeId::new([fill; 20])
    }

    fn make_node_info(fill: u8, port: u16) -> NodeInfo {
        NodeInfo {
            identity: NodeIdentity {
                node_id: node(fill),
                public_key: [fill; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([10, 0, 0, fill], port)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase2_default(),
        }
    }

    #[test]
    fn subnet_diversity_filter_limits_per_subnet() {
        let peers = vec![
            make_node_info(1, 4433), // 10.0.0.1
            make_node_info(2, 4433), // 10.0.0.2
            make_node_info(3, 4433), // 10.0.0.3 — same /24
        ];
        let filtered = apply_subnet_diversity(&peers, 2);
        assert_eq!(filtered.len(), 2); // max 2 per /24
    }

    #[test]
    fn subnet_diversity_allows_different_subnets() {
        let mut peers = vec![
            make_node_info(1, 4433), // 10.0.0.1
            make_node_info(2, 4433), // 10.0.0.2
        ];
        // Add peer from different subnet
        let mut different = make_node_info(3, 4433);
        different.addr = SocketAddr::from(([10, 0, 1, 3], 4433)); // 10.0.1.x
        peers.push(different);
        let filtered = apply_subnet_diversity(&peers, 2);
        assert_eq!(filtered.len(), 3); // all pass (2 from .0, 1 from .1)
    }

    fn make_institutional_node_info(fill: u8, port: u16) -> NodeInfo {
        NodeInfo {
            identity: NodeIdentity {
                node_id: node(fill),
                public_key: [fill; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([10, 0, fill, 1], port)),
            alt_addrs: vec![],
            capabilities: Capabilities::institutional_default(),
        }
    }

    #[test]
    fn institutional_diversity_limits_institutional_holders() {
        let mut peers = vec![
            make_institutional_node_info(1, 4433),
            make_institutional_node_info(2, 4434),
            make_institutional_node_info(3, 4435),
            make_institutional_node_info(4, 4436),
            make_node_info(5, 4437),
            make_node_info(6, 4438),
            make_node_info(7, 4439),
        ];
        // Different subnets for each
        for (i, peer) in peers.iter_mut().enumerate() {
            peer.addr = SocketAddr::from(([10, 0, i as u8, 1], 4433 + i as u16));
        }

        let filtered = apply_institutional_diversity(&peers, 7);
        let inst_count = filtered
            .iter()
            .filter(|p| p.capabilities.has(Capabilities::INSTITUTIONAL))
            .count();
        // max ceil(7 / 3.5) = 2
        assert!(inst_count <= 2, "got {inst_count} institutional, max 2");
        assert_eq!(filtered.len(), 5); // 2 institutional + 3 personal
    }

    #[test]
    fn institutional_diversity_allows_all_personal_when_no_institutional() {
        let peers: Vec<NodeInfo> = (1..=7u8)
            .map(|i| {
                let mut p = make_node_info(i, 4433);
                p.addr = SocketAddr::from(([10, 0, i, 1], 4433));
                p
            })
            .collect();

        let filtered = apply_institutional_diversity(&peers, 7);
        assert_eq!(filtered.len(), 7);
        let inst_count = filtered
            .iter()
            .filter(|p| p.capabilities.has(Capabilities::INSTITUTIONAL))
            .count();
        assert_eq!(inst_count, 0);
    }

    #[test]
    fn institutional_diversity_with_fewer_peers_than_target() {
        let peers = vec![
            make_institutional_node_info(1, 4433),
            make_node_info(2, 4434),
            make_node_info(3, 4435),
        ];

        let filtered = apply_institutional_diversity(&peers, 7);
        assert_eq!(filtered.len(), 3); // can't exceed available peers
    }

    #[test]
    fn extract_subnet_ipv4() {
        let addr: SocketAddr = "192.168.1.42:4433".parse().unwrap();
        assert_eq!(extract_subnet(&addr), "192.168.1");
    }
}
