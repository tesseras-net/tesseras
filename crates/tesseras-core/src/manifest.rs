use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::{ContentHash, CoreError, SchemaVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: String,
    pub hash: ContentHash,
    pub mime_type: String,
    pub size: u64,
}

impl fmt::Display for ManifestEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  {}  blake3:{}  {}  {}",
            self.path, self.hash, self.mime_type, self.size
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub version: SchemaVersion,
    pub created_at: DateTime<Utc>,
    pub creator: String,
    pub content_hash: ContentHash,
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, CoreError> {
        let mut lines = text.lines();

        // Parse header
        let header = lines
            .next()
            .ok_or_else(|| CoreError::InvalidManifest("empty input".into()))?;
        if header.trim() != "TESSERA MANIFEST v1" {
            return Err(CoreError::InvalidManifest(format!(
                "invalid header: {header}"
            )));
        }

        // Parse key-value pairs
        let mut created_at: Option<DateTime<Utc>> = None;
        let mut creator: Option<String> = None;
        let mut content_hash: Option<ContentHash> = None;

        for line in lines.by_ref() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "created" => {
                        created_at = Some(
                            DateTime::parse_from_rfc3339(value)
                                .map_err(|e| {
                                    CoreError::InvalidManifest(format!("invalid date: {e}"))
                                })?
                                .with_timezone(&Utc),
                        );
                    }
                    "creator" => {
                        creator = Some(value.to_string());
                    }
                    "content_hash" => {
                        content_hash = Some(ContentHash::from_str(value).map_err(|e| {
                            CoreError::InvalidManifest(format!("invalid content_hash: {e}"))
                        })?);
                    }
                    "encoding" | "schema" => { /* accepted but not stored separately */ }
                    _ => {
                        return Err(CoreError::InvalidManifest(format!("unknown key: {key}")));
                    }
                }
            }
        }

        let created_at = created_at
            .ok_or_else(|| CoreError::InvalidManifest("missing created field".into()))?;
        let creator =
            creator.ok_or_else(|| CoreError::InvalidManifest("missing creator field".into()))?;
        let content_hash = content_hash
            .ok_or_else(|| CoreError::InvalidManifest("missing content_hash field".into()))?;

        // Parse FILES: marker
        let files_marker = lines
            .next()
            .ok_or_else(|| CoreError::InvalidManifest("missing FILES: marker".into()))?;
        if files_marker.trim() != "FILES:" {
            return Err(CoreError::InvalidManifest(format!(
                "expected FILES:, got: {files_marker}"
            )));
        }

        // Parse entries
        let mut entries = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() != 4 {
                return Err(CoreError::InvalidManifest(format!(
                    "invalid entry (expected 4 fields, got {}): {trimmed}",
                    parts.len()
                )));
            }
            let path = parts[0].to_string();
            let hash_str = parts[1]
                .strip_prefix("blake3:")
                .ok_or_else(|| CoreError::InvalidManifest(format!("invalid hash prefix: {}", parts[1])))?;
            let hash = ContentHash::from_str(hash_str)
                .map_err(|e| CoreError::InvalidManifest(format!("invalid hash: {e}")))?;
            let mime_type = parts[2].to_string();
            let size: u64 = parts[3]
                .parse()
                .map_err(|e| CoreError::InvalidManifest(format!("invalid size: {e}")))?;
            entries.push(ManifestEntry {
                path,
                hash,
                mime_type,
                size,
            });
        }

        if entries.is_empty() {
            return Err(CoreError::InvalidManifest("no file entries".into()));
        }

        Ok(Manifest {
            version: SchemaVersion::V1,
            created_at,
            creator,
            content_hash,
            entries,
        })
    }
}

impl fmt::Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TESSERA MANIFEST v1")?;
        writeln!(
            f,
            "created: {}",
            self.created_at.format("%Y-%m-%dT%H:%M:%SZ")
        )?;
        writeln!(f, "creator: {}", self.creator)?;
        writeln!(f, "content_hash: {}", self.content_hash)?;
        writeln!(f, "encoding: UTF-8")?;
        writeln!(f, "schema: v1")?;
        writeln!(f)?;
        writeln!(f, "FILES:")?;
        for entry in &self.entries {
            writeln!(f, "{entry}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn sample_manifest_text() -> String {
        let hash = ContentHash::from_str(&"ab".repeat(32)).unwrap();
        let creator = "b3a7f2".to_string() + &"00".repeat(29);
        format!(
            "TESSERA MANIFEST v1\n\
             created: 2026-02-13T14:30:00Z\n\
             creator: {creator}\n\
             content_hash: {hash}\n\
             encoding: UTF-8\n\
             schema: v1\n\
             \n\
             FILES:\n\
             memories/a1b2c3/media.jpg  blake3:{hash}  image/jpeg  142032\n\
             memories/a1b2c3/context.txt  blake3:{hash}  text/plain  1847\n"
        )
    }

    #[test]
    fn manifest_parse_valid() {
        let text = sample_manifest_text();
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(manifest.version, SchemaVersion::V1);
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.entries[0].mime_type, "image/jpeg");
        assert_eq!(manifest.entries[0].size, 142032);
        assert_eq!(manifest.entries[1].mime_type, "text/plain");
    }

    #[test]
    fn manifest_serialize_parse_roundtrip() {
        let text = sample_manifest_text();
        let manifest = Manifest::parse(&text).unwrap();
        let serialized = manifest.to_string();
        let reparsed = Manifest::parse(&serialized).unwrap();
        assert_eq!(manifest.version, reparsed.version);
        assert_eq!(manifest.entries.len(), reparsed.entries.len());
        for (a, b) in manifest.entries.iter().zip(reparsed.entries.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.hash, b.hash);
            assert_eq!(a.mime_type, b.mime_type);
            assert_eq!(a.size, b.size);
        }
    }

    #[test]
    fn manifest_parse_rejects_missing_header() {
        let text = "not a manifest\n";
        assert!(Manifest::parse(text).is_err());
    }

    #[test]
    fn manifest_parse_rejects_empty_files() {
        let hash = ContentHash::from_str(&"ab".repeat(32)).unwrap();
        let text = format!(
            "TESSERA MANIFEST v1\n\
             created: 2026-02-13T14:30:00Z\n\
             creator: {}\n\
             content_hash: {hash}\n\
             encoding: UTF-8\n\
             schema: v1\n\
             \n\
             FILES:\n",
            "00".repeat(32)
        );
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn manifest_parse_rejects_truncated_entry() {
        let hash = ContentHash::from_str(&"ab".repeat(32)).unwrap();
        let text = format!(
            "TESSERA MANIFEST v1\n\
             created: 2026-02-13T14:30:00Z\n\
             creator: {}\n\
             content_hash: {hash}\n\
             encoding: UTF-8\n\
             schema: v1\n\
             \n\
             FILES:\n\
             memories/a1b2c3/media.jpg  blake3:{hash}\n",
            "00".repeat(32)
        );
        assert!(Manifest::parse(&text).is_err());
    }

    #[test]
    fn manifest_entry_displays_correctly() {
        let hash = ContentHash::from_str(&"ab".repeat(32)).unwrap();
        let entry = ManifestEntry {
            path: "memories/abc/media.jpg".to_string(),
            hash,
            mime_type: "image/jpeg".to_string(),
            size: 142032,
        };
        let line = entry.to_string();
        assert!(line.contains("memories/abc/media.jpg"));
        assert!(line.contains("blake3:"));
        assert!(line.contains("image/jpeg"));
        assert!(line.contains("142032"));
    }
}
