//! Prometheus metrics for the NAT traversal subsystems.
//!
//! Metrics are grouped by subsystem: STUN, punch, relay, reconnect.
//! All metrics use `tesseras_` prefix for consistent Prometheus naming.

use prometheus::{
    GaugeVec, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
};

/// All NAT traversal metrics, registered against a single [`Registry`].
pub struct NatMetrics {
    // --- STUN ---
    /// Current detected NAT type (gauge with `type` label).
    pub nat_type: GaugeVec,
    /// Total STUN binding requests sent.
    pub stun_requests_total: IntCounter,
    /// Total STUN binding failures (timeout, parse error, etc.).
    pub stun_failures_total: IntCounter,
    /// STUN round-trip latency in seconds.
    pub stun_latency_seconds: Histogram,

    // --- Punch ---
    /// Total punch attempts (labels: initiator_nat, target_nat).
    pub punch_attempts_total: IntCounterVec,
    /// Total successful punches (labels: initiator_nat, target_nat).
    pub punch_successes_total: IntCounterVec,
    /// Total failed punches (labels: initiator_nat, target_nat).
    pub punch_failures_total: IntCounterVec,
    /// Punch latency from intro to ready, in seconds.
    pub punch_latency_seconds: Histogram,

    // --- Relay ---
    /// Currently active relay sessions.
    pub relay_sessions_active: IntGauge,
    /// Total relay sessions created (labels: initiator_nat, target_nat).
    pub relay_sessions_total: IntCounterVec,
    /// Total bytes forwarded through relay.
    pub relay_bytes_forwarded: IntCounter,
    /// Relay sessions closed due to idle timeout.
    pub relay_idle_timeouts_total: IntCounter,
    /// Relay sessions closed due to rate limit.
    pub relay_rate_limited_total: IntCounter,

    // --- Connection Pool ---
    /// Current number of pooled connections.
    pub pool_size: IntGauge,
    /// Total pool cache hits.
    pub pool_hits_total: IntCounter,
    /// Total pool cache misses (new connection).
    pub pool_misses_total: IntCounter,
    /// Total pool evictions (LRU or idle reaper).
    pub pool_evictions_total: IntCounter,

    // --- Reconnect ---
    /// Total network change events detected.
    pub network_change_total: IntCounter,
    /// Total reconnection attempts by phase (label: phase).
    pub reconnect_attempts_total: IntCounterVec,
    /// Total successful reconnections by phase (label: phase).
    pub reconnect_successes_total: IntCounterVec,
    /// Reconnection duration in seconds.
    pub reconnect_duration_seconds: Histogram,
}

impl NatMetrics {
    /// Create and register all NAT traversal metrics in the given registry.
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let nat_type = GaugeVec::new(
            Opts::new(
                "tesseras_nat_type",
                "Current detected NAT type (1.0 = active)",
            ),
            &["type"],
        )?;
        registry.register(Box::new(nat_type.clone()))?;

        let stun_requests_total = IntCounter::new(
            "tesseras_stun_requests_total",
            "Total STUN binding requests sent",
        )?;
        registry.register(Box::new(stun_requests_total.clone()))?;

        let stun_failures_total = IntCounter::new(
            "tesseras_stun_failures_total",
            "Total STUN binding failures",
        )?;
        registry.register(Box::new(stun_failures_total.clone()))?;

        let stun_latency_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "tesseras_stun_latency_seconds",
                "STUN round-trip latency in seconds",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0]),
        )?;
        registry.register(Box::new(stun_latency_seconds.clone()))?;

        let nat_labels = &["initiator_nat", "target_nat"];

        let punch_attempts_total = IntCounterVec::new(
            Opts::new("tesseras_punch_attempts_total", "Total punch attempts"),
            nat_labels,
        )?;
        registry.register(Box::new(punch_attempts_total.clone()))?;

        let punch_successes_total = IntCounterVec::new(
            Opts::new("tesseras_punch_successes_total", "Total successful punches"),
            nat_labels,
        )?;
        registry.register(Box::new(punch_successes_total.clone()))?;

        let punch_failures_total = IntCounterVec::new(
            Opts::new("tesseras_punch_failures_total", "Total failed punches"),
            nat_labels,
        )?;
        registry.register(Box::new(punch_failures_total.clone()))?;

        let punch_latency_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "tesseras_punch_latency_seconds",
                "Punch latency from intro to ready in seconds",
            )
            .buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]),
        )?;
        registry.register(Box::new(punch_latency_seconds.clone()))?;

        let relay_sessions_active = IntGauge::new(
            "tesseras_relay_sessions_active",
            "Currently active relay sessions",
        )?;
        registry.register(Box::new(relay_sessions_active.clone()))?;

        let relay_sessions_total = IntCounterVec::new(
            Opts::new(
                "tesseras_relay_sessions_total",
                "Total relay sessions created",
            ),
            nat_labels,
        )?;
        registry.register(Box::new(relay_sessions_total.clone()))?;

        let relay_bytes_forwarded = IntCounter::new(
            "tesseras_relay_bytes_forwarded",
            "Total bytes forwarded through relay",
        )?;
        registry.register(Box::new(relay_bytes_forwarded.clone()))?;

        let relay_idle_timeouts_total = IntCounter::new(
            "tesseras_relay_idle_timeouts_total",
            "Relay sessions closed due to idle timeout",
        )?;
        registry.register(Box::new(relay_idle_timeouts_total.clone()))?;

        let relay_rate_limited_total = IntCounter::new(
            "tesseras_relay_rate_limited_total",
            "Relay sessions closed due to rate limit",
        )?;
        registry.register(Box::new(relay_rate_limited_total.clone()))?;

        let pool_size = IntGauge::new(
            "tesseras_conn_pool_size",
            "Current number of pooled connections",
        )?;
        registry.register(Box::new(pool_size.clone()))?;

        let pool_hits_total =
            IntCounter::new("tesseras_conn_pool_hits_total", "Total pool cache hits")?;
        registry.register(Box::new(pool_hits_total.clone()))?;

        let pool_misses_total =
            IntCounter::new("tesseras_conn_pool_misses_total", "Total pool cache misses")?;
        registry.register(Box::new(pool_misses_total.clone()))?;

        let pool_evictions_total =
            IntCounter::new("tesseras_conn_pool_evictions_total", "Total pool evictions")?;
        registry.register(Box::new(pool_evictions_total.clone()))?;

        let network_change_total = IntCounter::new(
            "tesseras_network_change_total",
            "Total network change events detected",
        )?;
        registry.register(Box::new(network_change_total.clone()))?;

        let reconnect_attempts_total = IntCounterVec::new(
            Opts::new(
                "tesseras_reconnect_attempts_total",
                "Total reconnection attempts by phase",
            ),
            &["phase"],
        )?;
        registry.register(Box::new(reconnect_attempts_total.clone()))?;

        let reconnect_successes_total = IntCounterVec::new(
            Opts::new(
                "tesseras_reconnect_successes_total",
                "Total successful reconnections by phase",
            ),
            &["phase"],
        )?;
        registry.register(Box::new(reconnect_successes_total.clone()))?;

        let reconnect_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "tesseras_reconnect_duration_seconds",
                "Reconnection duration in seconds",
            )
            .buckets(vec![0.5, 1.0, 2.0, 5.0, 10.0, 30.0]),
        )?;
        registry.register(Box::new(reconnect_duration_seconds.clone()))?;

        Ok(Self {
            nat_type,
            stun_requests_total,
            stun_failures_total,
            stun_latency_seconds,
            punch_attempts_total,
            punch_successes_total,
            punch_failures_total,
            punch_latency_seconds,
            relay_sessions_active,
            relay_sessions_total,
            relay_bytes_forwarded,
            relay_idle_timeouts_total,
            relay_rate_limited_total,
            pool_size,
            pool_hits_total,
            pool_misses_total,
            pool_evictions_total,
            network_change_total,
            reconnect_attempts_total,
            reconnect_successes_total,
            reconnect_duration_seconds,
        })
    }

    /// Set the current NAT type gauge (resets all type labels first).
    pub fn set_nat_type(&self, nat_type: &str) {
        for t in &["Public", "Cone", "Symmetric", "Unknown"] {
            self.nat_type.with_label_values(&[t]).set(0.0);
        }
        self.nat_type.with_label_values(&[nat_type]).set(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_register_and_increment() {
        let registry = Registry::new();
        let m = NatMetrics::new(&registry).unwrap();

        m.stun_requests_total.inc();
        assert_eq!(m.stun_requests_total.get(), 1);

        m.stun_failures_total.inc();
        assert_eq!(m.stun_failures_total.get(), 1);

        m.stun_latency_seconds.observe(0.123);
        assert_eq!(m.stun_latency_seconds.get_sample_count(), 1);
    }

    #[test]
    fn metrics_nat_type_gauge() {
        let registry = Registry::new();
        let m = NatMetrics::new(&registry).unwrap();

        m.set_nat_type("Cone");
        assert_eq!(m.nat_type.with_label_values(&["Cone"]).get(), 1.0);
        assert_eq!(m.nat_type.with_label_values(&["Public"]).get(), 0.0);

        m.set_nat_type("Public");
        assert_eq!(m.nat_type.with_label_values(&["Cone"]).get(), 0.0);
        assert_eq!(m.nat_type.with_label_values(&["Public"]).get(), 1.0);
    }

    #[test]
    fn metrics_punch_counters_with_labels() {
        let registry = Registry::new();
        let m = NatMetrics::new(&registry).unwrap();

        m.punch_attempts_total
            .with_label_values(&["Cone", "Symmetric"])
            .inc();
        assert_eq!(
            m.punch_attempts_total
                .with_label_values(&["Cone", "Symmetric"])
                .get(),
            1
        );

        m.punch_successes_total
            .with_label_values(&["Cone", "Symmetric"])
            .inc();
        m.punch_failures_total
            .with_label_values(&["Cone", "Cone"])
            .inc();
    }

    #[test]
    fn metrics_relay_session_tracking() {
        let registry = Registry::new();
        let m = NatMetrics::new(&registry).unwrap();

        m.relay_sessions_active.inc();
        m.relay_sessions_active.inc();
        assert_eq!(m.relay_sessions_active.get(), 2);

        m.relay_sessions_active.dec();
        assert_eq!(m.relay_sessions_active.get(), 1);

        m.relay_bytes_forwarded.inc_by(1500);
        assert_eq!(m.relay_bytes_forwarded.get(), 1500);
    }

    #[test]
    fn metrics_reconnect_counters() {
        let registry = Registry::new();
        let m = NatMetrics::new(&registry).unwrap();

        m.network_change_total.inc();
        assert_eq!(m.network_change_total.get(), 1);

        m.reconnect_attempts_total
            .with_label_values(&["QuicMigration"])
            .inc();
        m.reconnect_successes_total
            .with_label_values(&["QuicMigration"])
            .inc();
        m.reconnect_duration_seconds.observe(3.5);

        assert_eq!(
            m.reconnect_attempts_total
                .with_label_values(&["QuicMigration"])
                .get(),
            1
        );
    }

    #[test]
    fn metrics_double_register_fails() {
        let registry = Registry::new();
        let _m = NatMetrics::new(&registry).unwrap();
        assert!(NatMetrics::new(&registry).is_err());
    }
}
