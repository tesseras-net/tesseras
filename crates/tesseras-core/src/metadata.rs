use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{ApproximateDate, MemoryType, SchemaVersion};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub born_approximate: Option<ApproximateDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub version: SchemaVersion,
    pub created_at: DateTime<Utc>,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(default)]
    pub people: Vec<Person>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub language: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_serde_full() {
        let loc = Location {
            description: "São Paulo, Brasil".to_string(),
            coordinates: Some((-23.5505, -46.6333)),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let parsed: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.description, loc.description);
        assert_eq!(parsed.coordinates, loc.coordinates);
    }

    #[test]
    fn location_serde_no_coordinates() {
        let loc = Location {
            description: "somewhere".to_string(),
            coordinates: None,
        };
        let json = serde_json::to_string(&loc).unwrap();
        let parsed: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.coordinates, None);
    }

    #[test]
    fn person_serde_roundtrip() {
        let p = Person {
            name: "João".to_string(),
            relation: "creator".to_string(),
            born_approximate: Some(ApproximateDate::Year(1990)),
        };
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Person = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, p.name);
    }

    #[test]
    fn memory_metadata_serde_all_fields() {
        let meta = MemoryMetadata {
            version: SchemaVersion::V1,
            created_at: chrono::Utc::now(),
            memory_type: MemoryType::Reflection,
            location: Some(Location {
                description: "home".to_string(),
                coordinates: None,
            }),
            people: vec![Person {
                name: "Ana".to_string(),
                relation: "friend".to_string(),
                born_approximate: None,
            }],
            tags: vec!["cotidiano".to_string()],
            language: "pt-BR".to_string(),
            description: "A daily reflection".to_string(),
        };
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let parsed: MemoryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.memory_type, MemoryType::Reflection);
        assert_eq!(parsed.language, "pt-BR");
        assert_eq!(parsed.tags, vec!["cotidiano"]);
    }

    #[test]
    fn memory_metadata_serde_minimal() {
        let meta = MemoryMetadata {
            version: SchemaVersion::V1,
            created_at: chrono::Utc::now(),
            memory_type: MemoryType::Moment,
            location: None,
            people: vec![],
            tags: vec![],
            language: "en".to_string(),
            description: String::new(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: MemoryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.memory_type, MemoryType::Moment);
    }
}
