use crate::{ContentHash, MemoryMetadata};

#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub hash: ContentHash,
    pub media_path: String,
    pub context: Option<String>,
    pub metadata: MemoryMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryType, SchemaVersion};

    #[test]
    fn memory_creation() {
        let hash = ContentHash::new([0x01; 32]);
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
        let memory = Memory {
            hash,
            media_path: "media.jpg".to_string(),
            context: Some("A beautiful day".to_string()),
            metadata: meta,
        };
        assert_eq!(memory.hash, hash);
    }
}
