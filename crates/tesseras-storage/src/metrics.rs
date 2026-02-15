use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Registry};

pub struct StorageMetrics {
    // Fragment cache metrics (existing)
    pub fragment_cache_hits: IntCounter,
    pub fragment_cache_misses: IntCounter,
    pub fragment_cache_bytes: IntGauge,

    // CAS dedup metrics
    pub cas_objects_total: IntGauge,
    pub cas_bytes_total: IntGauge,
    pub cas_dedup_hits_total: IntCounter,
    pub cas_bytes_saved_total: IntCounter,

    // CAS GC metrics
    pub cas_gc_refcount_deletions_total: IntCounter,
    pub cas_gc_sweep_orphans_cleaned_total: IntCounter,
    pub cas_gc_sweep_leaked_refs_cleaned_total: IntCounter,
    pub cas_gc_sweep_skipped_young_total: IntCounter,
    pub cas_gc_sweep_duration_seconds: Histogram,
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

        let cas_objects =
            IntGauge::new("tesseras_cas_objects_total", "Total unique objects in CAS")?;
        let cas_bytes = IntGauge::new("tesseras_cas_bytes_total", "Total bytes on disk in CAS")?;
        let cas_dedup_hits = IntCounter::new(
            "tesseras_cas_dedup_hits_total",
            "Writes that found existing CAS object (dedup hit)",
        )?;
        let cas_bytes_saved = IntCounter::new(
            "tesseras_cas_bytes_saved_total",
            "Cumulative bytes not written due to dedup",
        )?;
        let cas_gc_refcount = IntCounter::new(
            "tesseras_cas_gc_refcount_deletions_total",
            "CAS objects removed via refcount reaching zero",
        )?;
        let cas_gc_orphans = IntCounter::new(
            "tesseras_cas_gc_sweep_orphans_cleaned_total",
            "Orphan files removed by CAS sweep",
        )?;
        let cas_gc_leaked = IntCounter::new(
            "tesseras_cas_gc_sweep_leaked_refs_cleaned_total",
            "Leaked refcount entries cleaned by CAS sweep",
        )?;
        let cas_gc_skipped_young = IntCounter::new(
            "tesseras_cas_gc_sweep_skipped_young_total",
            "Orphan files skipped due to grace period",
        )?;
        let cas_gc_duration = Histogram::with_opts(HistogramOpts::new(
            "tesseras_cas_gc_sweep_duration_seconds",
            "CAS sweep execution time in seconds",
        ))?;

        registry.register(Box::new(hits.clone()))?;
        registry.register(Box::new(misses.clone()))?;
        registry.register(Box::new(bytes.clone()))?;
        registry.register(Box::new(cas_objects.clone()))?;
        registry.register(Box::new(cas_bytes.clone()))?;
        registry.register(Box::new(cas_dedup_hits.clone()))?;
        registry.register(Box::new(cas_bytes_saved.clone()))?;
        registry.register(Box::new(cas_gc_refcount.clone()))?;
        registry.register(Box::new(cas_gc_orphans.clone()))?;
        registry.register(Box::new(cas_gc_leaked.clone()))?;
        registry.register(Box::new(cas_gc_skipped_young.clone()))?;
        registry.register(Box::new(cas_gc_duration.clone()))?;

        Ok(Self {
            fragment_cache_hits: hits,
            fragment_cache_misses: misses,
            fragment_cache_bytes: bytes,
            cas_objects_total: cas_objects,
            cas_bytes_total: cas_bytes,
            cas_dedup_hits_total: cas_dedup_hits,
            cas_bytes_saved_total: cas_bytes_saved,
            cas_gc_refcount_deletions_total: cas_gc_refcount,
            cas_gc_sweep_orphans_cleaned_total: cas_gc_orphans,
            cas_gc_sweep_leaked_refs_cleaned_total: cas_gc_leaked,
            cas_gc_sweep_skipped_young_total: cas_gc_skipped_young,
            cas_gc_sweep_duration_seconds: cas_gc_duration,
        })
    }
}
