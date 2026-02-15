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

/// Encryption metadata stored in the manifest for sealed/private tesseras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEncryption {
    /// Encryption scheme identifier (e.g., "hybrid-kem-v1").
    pub scheme: String,
    /// Base64-encoded SealedKeyEnvelope (MessagePack serialized).
    pub envelope_base64: String,
    /// When the tessera can be opened (None for Private).
    pub open_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub version: SchemaVersion,
    pub created_at: DateTime<Utc>,
    pub creator: String,
    pub content_hash: ContentHash,
    pub entries: Vec<ManifestEntry>,
    pub encryption: Option<ManifestEncryption>,
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

        let created_at =
            created_at.ok_or_else(|| CoreError::InvalidManifest("missing created field".into()))?;
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

        // Collect remaining lines for entries + optional encryption
        let remaining: Vec<&str> = lines.collect();
        let mut entries = Vec::new();
        let mut encryption = None;
        let mut i = 0;

        // Parse file entries
        while i < remaining.len() {
            let trimmed = remaining[i].trim();
            i += 1;
            if trimmed.is_empty() {
                continue;
            }
            if trimmed == "ENCRYPTION:" {
                // Parse encryption block from remaining lines
                let mut scheme = None;
                let mut envelope = None;
                let mut open_after = None;
                while i < remaining.len() {
                    let etrimmed = remaining[i].trim();
                    i += 1;
                    if etrimmed.is_empty() {
                        continue;
                    }
                    if let Some((key, value)) = etrimmed.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();
                        match key {
                            "scheme" => scheme = Some(value.to_string()),
                            "envelope" => envelope = Some(value.to_string()),
                            "open_after" => {
                                open_after = Some(
                                    DateTime::parse_from_rfc3339(value)
                                        .map_err(|e| {
                                            CoreError::InvalidManifest(format!(
                                                "invalid open_after date: {e}"
                                            ))
                                        })?
                                        .with_timezone(&Utc),
                                );
                            }
                            _ => {
                                return Err(CoreError::InvalidManifest(format!(
                                    "unknown encryption key: {key}"
                                )));
                            }
                        }
                    }
                }
                let scheme = scheme.ok_or_else(|| {
                    CoreError::InvalidManifest("missing encryption scheme".into())
                })?;
                let envelope_base64 = envelope.ok_or_else(|| {
                    CoreError::InvalidManifest("missing encryption envelope".into())
                })?;
                encryption = Some(ManifestEncryption {
                    scheme,
                    envelope_base64,
                    open_after,
                });
                break;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() != 4 {
                return Err(CoreError::InvalidManifest(format!(
                    "invalid entry (expected 4 fields, got {}): {trimmed}",
                    parts.len()
                )));
            }
            let path = parts[0].to_string();
            let hash_str = parts[1].strip_prefix("blake3:").ok_or_else(|| {
                CoreError::InvalidManifest(format!("invalid hash prefix: {}", parts[1]))
            })?;
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
            encryption,
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

        if let Some(enc) = &self.encryption {
            writeln!(f)?;
            writeln!(f, "ENCRYPTION:")?;
            writeln!(f, "  scheme: {}", enc.scheme)?;
            writeln!(f, "  envelope: {}", enc.envelope_base64)?;
            if let Some(open_after) = &enc.open_after {
                writeln!(
                    f,
                    "  open_after: {}",
                    open_after.format("%Y-%m-%dT%H:%M:%SZ")
                )?;
            }
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

    #[test]
    fn manifest_with_encryption_block_roundtrip() {
        let hash = ContentHash::from_str(&"ab".repeat(32)).unwrap();
        let creator = "00".repeat(32);
        let manifest = Manifest {
            version: SchemaVersion::V1,
            created_at: chrono::Utc::now(),
            creator,
            content_hash: hash,
            entries: vec![ManifestEntry {
                path: "memories/a1b2c3/encrypted.bin".to_string(),
                hash,
                mime_type: "application/octet-stream".to_string(),
                size: 1024,
            }],
            encryption: Some(ManifestEncryption {
                scheme: "hybrid-kem-v1".to_string(),
                envelope_base64: "dGVzdGVudmVsb3Bl".to_string(),
                open_after: Some(
                    chrono::DateTime::parse_from_rfc3339("2050-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                ),
            }),
        };
        let text = manifest.to_string();
        assert!(text.contains("ENCRYPTION:"));
        assert!(text.contains("scheme: hybrid-kem-v1"));
        assert!(text.contains("envelope: dGVzdGVudmVsb3Bl"));
        assert!(text.contains("open_after: 2050-01-01T00:00:00Z"));

        let reparsed = Manifest::parse(&text).unwrap();
        let enc = reparsed.encryption.unwrap();
        assert_eq!(enc.scheme, "hybrid-kem-v1");
        assert_eq!(enc.envelope_base64, "dGVzdGVudmVsb3Bl");
        assert!(enc.open_after.is_some());
    }

    #[test]
    fn manifest_without_encryption_roundtrip() {
        let text = sample_manifest_text();
        let manifest = Manifest::parse(&text).unwrap();
        assert!(manifest.encryption.is_none());
    }

    mod proptests {
        use super::*;
        use crate::SchemaVersion;
        use proptest::prelude::*;

        fn arb_manifest_entry() -> impl Strategy<Value = ManifestEntry> {
            (
                "[a-z]{3,10}/[a-z]{3,10}\\.[a-z]{3}",
                proptest::array::uniform32(any::<u8>()),
                prop_oneof![
                    Just("image/jpeg".to_string()),
                    Just("text/plain".to_string()),
                    Just("audio/wav".to_string()),
                    Just("video/webm".to_string()),
                ],
                1u64..10_000_000,
            )
                .prop_map(|(path, hash_bytes, mime, size)| ManifestEntry {
                    path,
                    hash: ContentHash::new(hash_bytes),
                    mime_type: mime,
                    size,
                })
        }

        proptest! {
            #[test]
            fn manifest_serialize_parse_roundtrip_prop(
                entries in proptest::collection::vec(arb_manifest_entry(), 1..5),
                content_hash in proptest::array::uniform32(any::<u8>()),
            ) {
                let manifest = Manifest {
                    version: SchemaVersion::V1,
                    created_at: chrono::Utc::now(),
                    creator: "aa".repeat(32),
                    content_hash: ContentHash::new(content_hash),
                    entries,
                    encryption: None,
                };
                let text = manifest.to_string();
                let reparsed = Manifest::parse(&text).unwrap();
                prop_assert_eq!(manifest.entries.len(), reparsed.entries.len());
                for (a, b) in manifest.entries.iter().zip(reparsed.entries.iter()) {
                    prop_assert_eq!(&a.path, &b.path);
                    prop_assert_eq!(a.hash, b.hash);
                    prop_assert_eq!(&a.mime_type, &b.mime_type);
                    prop_assert_eq!(a.size, b.size);
                }
            }
        }
    }
}
