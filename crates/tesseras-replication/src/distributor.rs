use std::collections::HashMap;
use std::net::SocketAddr;

use tesseras_core::NodeInfo;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::types::NodeId;
    use tesseras_core::{Capabilities, NodeIdentity};
    use std::net::SocketAddr;

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

    #[test]
    fn extract_subnet_ipv4() {
        let addr: SocketAddr = "192.168.1.42:4433".parse().unwrap();
        assert_eq!(extract_subnet(&addr), "192.168.1");
    }
}
