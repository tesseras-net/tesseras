use prometheus::{IntCounter, IntGauge, Registry};

pub struct StorageMetrics {
    pub fragment_cache_hits: IntCounter,
    pub fragment_cache_misses: IntCounter,
    pub fragment_cache_bytes: IntGauge,
}

impl StorageMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let hits = IntCounter::new(
            "tesseras_fragment_cache_hits_total",
            "Number of fragment cache hits",
        )?;
        let misses = IntCounter::new(
            "tesseras_fragment_cache_misses_total",
            "Number of fragment cache misses",
        )?;
        let bytes = IntGauge::new(
            "tesseras_fragment_cache_bytes",
            "Current fragment cache size in bytes",
        )?;
        registry.register(Box::new(hits.clone()))?;
        registry.register(Box::new(misses.clone()))?;
        registry.register(Box::new(bytes.clone()))?;
        Ok(Self {
            fragment_cache_hits: hits,
            fragment_cache_misses: misses,
            fragment_cache_bytes: bytes,
        })
    }
}
