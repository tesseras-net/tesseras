use prometheus::{Gauge, IntCounter, IntGauge, Registry};

/// Prometheus metrics for institutional node monitoring.
#[allow(dead_code)]
pub struct InstitutionalMetrics {
    pub pledge_bytes: IntGauge,
    pub stored_bytes: IntGauge,
    pub pledge_utilization_ratio: Gauge,
    pub peers_served: IntGauge,
    pub search_index_total: IntGauge,
    pub search_queries_total: IntCounter,
    pub dns_verification_status: IntGauge,
    pub dns_verification_last: IntGauge,
}

#[allow(dead_code)]
impl InstitutionalMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let pledge_bytes = IntGauge::new(
            "tesseras_institutional_pledge_bytes",
            "Configured storage pledge in bytes",
        )?;
        let stored_bytes = IntGauge::new(
            "tesseras_institutional_stored_bytes",
            "Bytes currently stored for other peers",
        )?;
        let pledge_utilization_ratio = Gauge::new(
            "tesseras_institutional_pledge_utilization_ratio",
            "Ratio of stored bytes to pledge bytes (0.0–1.0+)",
        )?;
        let peers_served = IntGauge::new(
            "tesseras_institutional_peers_served",
            "Number of distinct peers this node stores fragments for",
        )?;
        let search_index_total = IntGauge::new(
            "tesseras_institutional_search_index_total",
            "Number of tesseras indexed for search",
        )?;
        let search_queries_total = IntCounter::new(
            "tesseras_institutional_search_queries_total",
            "Total search queries processed",
        )?;
        let dns_verification_status = IntGauge::new(
            "tesseras_institutional_dns_verification_status",
            "DNS verification status (1=verified, 0=failed)",
        )?;
        let dns_verification_last = IntGauge::new(
            "tesseras_institutional_dns_verification_last_timestamp",
            "Unix timestamp of last DNS verification attempt",
        )?;

        registry.register(Box::new(pledge_bytes.clone()))?;
        registry.register(Box::new(stored_bytes.clone()))?;
        registry.register(Box::new(pledge_utilization_ratio.clone()))?;
        registry.register(Box::new(peers_served.clone()))?;
        registry.register(Box::new(search_index_total.clone()))?;
        registry.register(Box::new(search_queries_total.clone()))?;
        registry.register(Box::new(dns_verification_status.clone()))?;
        registry.register(Box::new(dns_verification_last.clone()))?;

        Ok(Self {
            pledge_bytes,
            stored_bytes,
            pledge_utilization_ratio,
            peers_served,
            search_index_total,
            search_queries_total,
            dns_verification_status,
            dns_verification_last,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_register_without_conflict() {
        let registry = Registry::new();
        let metrics = InstitutionalMetrics::new(&registry).unwrap();
        metrics.pledge_bytes.set(1_000_000_000);
        metrics.dns_verification_status.set(1);
        assert_eq!(metrics.pledge_bytes.get(), 1_000_000_000);
        assert_eq!(metrics.dns_verification_status.get(), 1);
    }

    #[test]
    fn utilization_ratio_tracks_pledge() {
        let registry = Registry::new();
        let metrics = InstitutionalMetrics::new(&registry).unwrap();
        metrics.pledge_bytes.set(100);
        metrics.stored_bytes.set(75);
        let ratio = metrics.stored_bytes.get() as f64 / metrics.pledge_bytes.get() as f64;
        metrics.pledge_utilization_ratio.set(ratio);
        assert!((metrics.pledge_utilization_ratio.get() - 0.75).abs() < f64::EPSILON);
    }
}
