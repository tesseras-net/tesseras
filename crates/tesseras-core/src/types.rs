use crate::error::CoreError;

macro_rules! hex_newtype {
    ($name:ident, $len:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) [u8; $len]);

        impl $name {
            pub fn new(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }
            pub fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for byte in &self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl std::str::FromStr for $name {
            type Err = CoreError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s.len() != $len * 2 {
                    return Err(CoreError::InvalidLength {
                        expected: $len * 2,
                        got: s.len(),
                    });
                }
                let mut bytes = [0u8; $len];
                for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                    let hex_str = std::str::from_utf8(chunk)
                        .map_err(|_| CoreError::InvalidHex(s.to_string()))?;
                    bytes[i] = u8::from_str_radix(hex_str, 16)
                        .map_err(|_| CoreError::InvalidHex(s.to_string()))?;
                }
                Ok(Self(bytes))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

hex_newtype!(ContentHash, 32);
hex_newtype!(NodeId, 20);

impl ContentHash {
    pub fn to_base32(&self) -> String {
        crate::crockford::encode(&self.0)
    }

    pub fn to_base32_short(&self, n: usize) -> String {
        let full = self.to_base32();
        full[..n.min(full.len())].to_string()
    }
}

/// User-supplied hash input: full hash or prefix (base32 or hex).
#[derive(Debug, Clone)]
pub enum HashPrefix {
    /// Full 32-byte hash (parsed from 64 hex or 52 base32).
    Exact(ContentHash),
    /// Hex prefix (lowercase), for direct SQL query.
    HexPrefix(String),
    /// Base32 prefix with derived hex prefix + normalized base32 for post-filter.
    Base32Prefix {
        hex_prefix: String,
        base32_prefix: String,
    },
}

impl HashPrefix {
    /// Parse user input into a `HashPrefix`.
    ///
    /// Rules:
    /// - 64 hex chars → `Exact` (parsed as hex)
    /// - 52 valid base32 chars → `Exact` (decoded from base32)
    /// - Contains Crockford-exclusive char (G-Z except I,L,O,U) → `Base32Prefix`
    /// - All hex-valid chars, < 64 → `HexPrefix`
    /// - Otherwise → error
    pub fn parse(input: &str) -> Result<Self, CoreError> {
        let trimmed = input.trim();

        // Full 64 hex chars → exact hex
        if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            let hash = ContentHash::from_str(trimmed)
                .map_err(|_| CoreError::InvalidHashPrefix(trimmed.to_string()))?;
            return Ok(HashPrefix::Exact(hash));
        }

        // Check if all chars are valid Crockford
        let all_crockford = trimmed.bytes().all(|b| {
            b < 128 && b != b'U' && b != b'u' && crate::crockford::DECODE_TABLE[b as usize] != 0xFF
        });

        // Full 52 valid base32 chars → exact base32
        if trimmed.len() == 52 && all_crockford {
            if let Some(decoded) = crate::crockford::decode(trimmed) {
                if decoded.len() >= 32 {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&decoded[..32]);
                    return Ok(HashPrefix::Exact(ContentHash::new(bytes)));
                }
            }
        }

        // Check for Crockford-exclusive characters (not valid in hex)
        let has_base32_exclusive = trimmed.bytes().any(|b| {
            matches!(
                b.to_ascii_uppercase(),
                b'G' | b'H'
                    | b'J'
                    | b'K'
                    | b'M'
                    | b'N'
                    | b'P'
                    | b'Q'
                    | b'R'
                    | b'S'
                    | b'T'
                    | b'V'
                    | b'W'
                    | b'X'
                    | b'Y'
                    | b'Z'
            )
        });

        if has_base32_exclusive {
            if !all_crockford {
                return Err(CoreError::InvalidHashPrefix(trimmed.to_string()));
            }
            let base32_prefix = crate::crockford::normalize(trimmed)
                .ok_or_else(|| CoreError::InvalidHashPrefix(trimmed.to_string()))?;
            let hex_prefix = crate::crockford::prefix_to_hex_prefix(trimmed)
                .ok_or_else(|| CoreError::InvalidHashPrefix(trimmed.to_string()))?;
            return Ok(HashPrefix::Base32Prefix {
                hex_prefix,
                base32_prefix,
            });
        }

        // All chars are hex-valid (0-9, a-f, A-F)
        let all_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());
        if all_hex && !trimmed.is_empty() {
            return Ok(HashPrefix::HexPrefix(trimmed.to_ascii_lowercase()));
        }

        Err(CoreError::InvalidHashPrefix(trimmed.to_string()))
    }
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn content_hash_display_fromstr_roundtrip() {
        let bytes = [0xab; 32];
        let hash = ContentHash(bytes);
        let hex = hash.to_string();
        assert_eq!(hex, "ab".repeat(32));
        let parsed = ContentHash::from_str(&hex).unwrap();
        assert_eq!(parsed, hash);
    }

    #[test]
    fn content_hash_fromstr_rejects_short() {
        assert!(ContentHash::from_str("abcd").is_err());
    }

    #[test]
    fn content_hash_fromstr_rejects_invalid_hex() {
        let bad = "zz".repeat(32);
        assert!(ContentHash::from_str(&bad).is_err());
    }

    #[test]
    fn content_hash_serde_roundtrip() {
        let hash = ContentHash([0x42; 32]);
        let json = serde_json::to_string(&hash).unwrap();
        let parsed: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, hash);
        // Serializes as hex string, not byte array
        assert!(json.contains(&"42".repeat(32)));
    }

    #[test]
    fn node_id_display_fromstr_roundtrip() {
        let bytes = [0xcd; 20];
        let id = NodeId(bytes);
        let hex = id.to_string();
        assert_eq!(hex, "cd".repeat(20));
        let parsed = NodeId::from_str(&hex).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn node_id_serde_roundtrip() {
        let id = NodeId([0x07; 20]);
        let json = serde_json::to_string(&id).unwrap();
        let parsed: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn content_hash_hex_roundtrip(bytes in proptest::array::uniform32(any::<u8>())) {
                let hash = ContentHash::new(bytes);
                let hex = hash.to_string();
                let parsed: ContentHash = hex.parse().unwrap();
                prop_assert_eq!(parsed, hash);
            }

            #[test]
            fn node_id_hex_roundtrip(bytes in proptest::array::uniform20(any::<u8>())) {
                let id = NodeId::new(bytes);
                let hex = id.to_string();
                let parsed: NodeId = hex.parse().unwrap();
                prop_assert_eq!(parsed, id);
            }
        }
    }
}
