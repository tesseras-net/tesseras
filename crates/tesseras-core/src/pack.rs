//! Pack/unpack a tessera's files into a single byte buffer for replication.
//!
//! Format: MessagePack-encoded `Vec<PackedFile>` where each entry is
//! (relative_path, file_contents). Paths use forward slashes.

use serde::{Deserialize, Serialize};

/// A single file entry in a packed tessera.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedFile {
    pub path: String,
    pub data: Vec<u8>,
}

/// Pack a list of files into a single byte buffer.
pub fn pack(files: &[PackedFile]) -> Vec<u8> {
    rmp_serde::to_vec(files).expect("PackedFile serialization cannot fail")
}

/// Unpack a byte buffer into a list of files.
pub fn unpack(data: &[u8]) -> Result<Vec<PackedFile>, PackError> {
    rmp_serde::from_slice(data).map_err(|e| PackError::Deserialize(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("failed to deserialize packed tessera: {0}")]
    Deserialize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_roundtrip() {
        let files = vec![
            PackedFile {
                path: "MANIFEST".into(),
                data: b"TESSERA MANIFEST v1\n".to_vec(),
            },
            PackedFile {
                path: "memories/abc123/media.jpg".into(),
                data: vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG header
            },
            PackedFile {
                path: "memories/abc123/context.txt".into(),
                data: b"A sunny day".to_vec(),
            },
        ];

        let packed = pack(&files);
        let unpacked = unpack(&packed).unwrap();

        assert_eq!(unpacked.len(), 3);
        assert_eq!(unpacked[0].path, "MANIFEST");
        assert_eq!(unpacked[0].data, b"TESSERA MANIFEST v1\n");
        assert_eq!(unpacked[1].path, "memories/abc123/media.jpg");
        assert_eq!(unpacked[2].data, b"A sunny day");
    }

    #[test]
    fn unpack_rejects_garbage() {
        let result = unpack(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn pack_empty_is_valid() {
        let packed = pack(&[]);
        let unpacked = unpack(&packed).unwrap();
        assert!(unpacked.is_empty());
    }
}
