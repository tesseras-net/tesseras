use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// BLAKE3 content hash, 32 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", &hex::encode(self.0)[..12])
    }
}

impl FromStr for ContentHash {
    type Err = ContentHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ContentHashError::InvalidHex)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ContentHashError::InvalidLength)?;
        Ok(Self(arr))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContentHashError {
    #[error("invalid hex string")]
    InvalidHex,
    #[error("expected 32 bytes, got wrong length")]
    InvalidLength,
}

/// Kademlia node ID, 32 bytes. Derived from BLAKE3(public_key).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// XOR distance for Kademlia routing.
    pub fn distance(&self, other: &NodeId) -> [u8; 32] {
        let mut result = [0u8; 32];
        for (i, byte) in result.iter_mut().enumerate() {
            *byte = self.0[i] ^ other.0[i];
        }
        result
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", &hex::encode(self.0)[..12])
    }
}

impl FromStr for NodeId {
    type Err = ContentHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ContentHashError::InvalidHex)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ContentHashError::InvalidLength)?;
        Ok(Self(arr))
    }
}

/// Visibility of a tessera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Circle { name: String },
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Private => write!(f, "private"),
            Visibility::Circle { name } => write!(f, "circle:{name}"),
        }
    }
}

impl FromStr for Visibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Visibility::Public),
            "private" => Ok(Visibility::Private),
            s if s.starts_with("circle:") => Ok(Visibility::Circle {
                name: s[7..].to_string(),
            }),
            _ => Err(format!("unknown visibility: {s}")),
        }
    }
}

/// Media type of a memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Text,
}

impl MediaType {
    /// Detect media type from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => MediaType::Image,
            "wav" | "mp3" | "flac" | "ogg" | "aac" => MediaType::Audio,
            "mp4" | "webm" | "mkv" | "avi" | "mov" => MediaType::Video,
            _ => MediaType::Text,
        }
    }
}

/// A single memory (file) within a tessera.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub filename: String,
    pub media_type: MediaType,
    pub size: u64,
    pub blob_hash: ContentHash,
}

/// A tessera — a signed, self-contained package of memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tessera {
    pub hash: ContentHash,
    pub author: Vec<u8>,
    pub signature: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub name: Option<String>,
    pub visibility: Visibility,
    pub memories: Vec<Memory>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_roundtrip() {
        let hash = ContentHash::new([0xab; 32]);
        let s = hash.to_string();
        let parsed: ContentHash = s.parse().unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn content_hash_invalid_hex() {
        let result: Result<ContentHash, _> = "not-hex".parse();
        assert!(result.is_err());
    }

    #[test]
    fn content_hash_wrong_length() {
        let result: Result<ContentHash, _> = "abcd".parse();
        assert!(result.is_err());
    }

    #[test]
    fn node_id_distance() {
        let a = NodeId::new([0xFF; 32]);
        let b = NodeId::new([0x00; 32]);
        assert_eq!(a.distance(&b), [0xFF; 32]);

        let c = NodeId::new([0xFF; 32]);
        assert_eq!(a.distance(&c), [0x00; 32]);
    }

    #[test]
    fn visibility_roundtrip() {
        assert_eq!("public".parse::<Visibility>().unwrap(), Visibility::Public);
        assert_eq!(
            "circle:family".parse::<Visibility>().unwrap(),
            Visibility::Circle {
                name: "family".into()
            }
        );
        assert_eq!(
            Visibility::Circle {
                name: "friends".into()
            }
            .to_string(),
            "circle:friends"
        );
    }

    #[test]
    fn media_type_detection() {
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Image);
        assert_eq!(MediaType::from_extension("MP4"), MediaType::Video);
        assert_eq!(MediaType::from_extension("wav"), MediaType::Audio);
        assert_eq!(MediaType::from_extension("rs"), MediaType::Text);
    }
}
