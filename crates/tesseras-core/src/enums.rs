use std::fmt;

use chrono::NaiveDate;
#[cfg(feature = "experimental-visibility")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::ContentHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Moment,
    Reflection,
    Daily,
    Relation,
    Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Circle {
        circle: String,
    },
    Public,
    #[cfg(feature = "experimental-visibility")]
    PublicAfterDeath {
        inactive_years: u32,
    },
    #[cfg(feature = "experimental-visibility")]
    Sealed {
        open_after: DateTime<Utc>,
    },
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Private => write!(f, "private"),
            Visibility::Circle { circle } => write!(f, "circle:{circle}"),
            Visibility::Public => write!(f, "public"),
            #[cfg(feature = "experimental-visibility")]
            Visibility::PublicAfterDeath { inactive_years } => {
                write!(f, "public after {inactive_years}y inactive")
            }
            #[cfg(feature = "experimental-visibility")]
            Visibility::Sealed { open_after } => {
                write!(f, "sealed until {}", open_after.format("%Y-%m-%d"))
            }
        }
    }
}

/// Context bound into AES-GCM authenticated data (AAD) to prevent ciphertext swapping.
///
/// For `Sealed`, includes `open_after` timestamp — moving ciphertext from a tessera
/// sealed until 2050 into one sealed until 2025 causes decryption failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionContext {
    Private {
        content_hash: ContentHash,
    },
    Circle {
        content_hash: ContentHash,
        circle: String,
    },
    #[cfg(feature = "experimental-visibility")]
    Sealed {
        content_hash: ContentHash,
        open_after: DateTime<Utc>,
    },
}

impl EncryptionContext {
    /// Deterministic serialization for use as AAD.
    pub fn to_aad_bytes(&self) -> Vec<u8> {
        match self {
            Self::Private { content_hash } => {
                let mut buf = vec![0x00]; // tag byte for Private
                buf.extend_from_slice(content_hash.as_bytes());
                buf
            }
            Self::Circle {
                content_hash,
                circle,
            } => {
                let mut buf = vec![0x02]; // tag byte for Circle
                buf.extend_from_slice(content_hash.as_bytes());
                buf.extend_from_slice(circle.as_bytes());
                buf
            }
            #[cfg(feature = "experimental-visibility")]
            Self::Sealed {
                content_hash,
                open_after,
            } => {
                let mut buf = vec![0x01]; // tag byte for Sealed
                buf.extend_from_slice(content_hash.as_bytes());
                buf.extend_from_slice(&open_after.timestamp().to_le_bytes());
                buf
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ApproximateDate {
    Year(u16),
    YearMonth(u16, u8),
    Full(NaiveDate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaVersion {
    V1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_type_serde_roundtrip() {
        for mt in [
            MemoryType::Moment,
            MemoryType::Reflection,
            MemoryType::Daily,
            MemoryType::Relation,
            MemoryType::Object,
        ] {
            let json = serde_json::to_string(&mt).unwrap();
            let parsed: MemoryType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, mt);
        }
    }

    #[test]
    fn memory_type_serializes_lowercase() {
        let json = serde_json::to_string(&MemoryType::Moment).unwrap();
        assert_eq!(json, "\"moment\"");
    }

    #[test]
    fn visibility_private_serde() {
        let v = Visibility::Private;
        let json = serde_json::to_string(&v).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn visibility_circle_serde() {
        let v = Visibility::Circle {
            circle: "family".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("family"));
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn visibility_circle_display() {
        let v = Visibility::Circle {
            circle: "family".to_string(),
        };
        assert_eq!(v.to_string(), "circle:family");
    }

    #[cfg(feature = "experimental-visibility")]
    #[test]
    fn visibility_sealed_serde_rfc3339() {
        let dt = chrono::DateTime::parse_from_rfc3339("2050-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let v = Visibility::Sealed { open_after: dt };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("2050"));
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[cfg(feature = "experimental-visibility")]
    #[test]
    fn visibility_public_after_death_serde() {
        let v = Visibility::PublicAfterDeath { inactive_years: 5 };
        let json = serde_json::to_string(&v).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn approximate_date_year_serde() {
        let d = ApproximateDate::Year(1990);
        let json = serde_json::to_string(&d).unwrap();
        let parsed: ApproximateDate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, d);
    }

    #[test]
    fn approximate_date_year_month_serde() {
        let d = ApproximateDate::YearMonth(1990, 6);
        let json = serde_json::to_string(&d).unwrap();
        let parsed: ApproximateDate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, d);
    }

    #[test]
    fn approximate_date_full_serde() {
        let d = ApproximateDate::Full(chrono::NaiveDate::from_ymd_opt(1990, 6, 15).unwrap());
        let json = serde_json::to_string(&d).unwrap();
        let parsed: ApproximateDate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, d);
    }

    #[test]
    fn schema_version_serde() {
        let v = SchemaVersion::V1;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"v1\"");
        let parsed: SchemaVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }
}
