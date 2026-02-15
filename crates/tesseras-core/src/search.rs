use serde::{Deserialize, Serialize};

use crate::enums::{MemoryType, Visibility};
use crate::types::ContentHash;

/// Geographic filter for search queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeoFilter {
    BoundingBox {
        min_lat: f64,
        min_lon: f64,
        max_lat: f64,
        max_lon: f64,
    },
    Radius {
        lat: f64,
        lon: f64,
        radius_km: f64,
    },
}

/// Date range filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Filters for a SEARCH query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchFilters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<MemoryType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<GeoFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Excerpt of tessera metadata returned in search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataExcerpt {
    pub title: Option<String>,
    pub description: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub visibility: Visibility,
    pub language: Option<String>,
    pub tags: Vec<String>,
}

/// A single search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub hash: ContentHash,
    pub metadata: MetadataExcerpt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_filters_default_is_empty() {
        let filters = SearchFilters::default();
        assert!(filters.memory_type.is_none());
        assert!(filters.date_range.is_none());
        assert!(filters.location.is_none());
        assert!(filters.tags.is_none());
        assert!(filters.language.is_none());
    }

    #[test]
    fn search_filters_serde_roundtrip() {
        let filters = SearchFilters {
            memory_type: Some(MemoryType::Moment),
            date_range: Some(DateRange {
                from: Some(chrono::Utc::now()),
                to: None,
            }),
            location: Some(GeoFilter::Radius {
                lat: -23.5505,
                lon: -46.6333,
                radius_km: 50.0,
            }),
            tags: Some(vec!["cotidiano".into(), "cidade".into()]),
            language: Some("pt-BR".into()),
        };
        let bytes = rmp_serde::to_vec(&filters).unwrap();
        let parsed: SearchFilters = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed.memory_type, filters.memory_type);
        assert_eq!(parsed.language, filters.language);
        assert!(parsed.location.is_some());
    }

    #[test]
    fn search_hit_serde_roundtrip() {
        let hit = SearchHit {
            hash: ContentHash::new([0xab; 32]),
            metadata: MetadataExcerpt {
                title: Some("Test memory".into()),
                description: Some("A test".into()),
                memory_type: Some(MemoryType::Reflection),
                created_at: Some(chrono::Utc::now()),
                visibility: Visibility::Public,
                language: Some("en".into()),
                tags: vec!["test".into()],
            },
        };
        let bytes = rmp_serde::to_vec(&hit).unwrap();
        let parsed: SearchHit = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed.hash, hit.hash);
        assert_eq!(parsed.metadata.title, Some("Test memory".into()));
    }

    #[test]
    fn geo_filter_bounding_box_serde() {
        let filter = GeoFilter::BoundingBox {
            min_lat: -24.0,
            min_lon: -47.0,
            max_lat: -23.0,
            max_lon: -46.0,
        };
        let bytes = rmp_serde::to_vec(&filter).unwrap();
        let parsed: GeoFilter = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed, filter);
    }
}
