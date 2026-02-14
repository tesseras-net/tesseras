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
