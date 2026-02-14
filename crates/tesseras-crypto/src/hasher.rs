use std::io::Read;

use tesseras_core::ContentHash;

use crate::CryptoError;

pub struct Blake3Hasher;

impl Blake3Hasher {
    pub fn hash(data: &[u8]) -> ContentHash {
        let hash = blake3::hash(data);
        ContentHash::new(*hash.as_bytes())
    }

    pub fn hash_reader(reader: &mut impl Read) -> Result<ContentHash, CryptoError> {
        let mut hasher = blake3::Hasher::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let hash = hasher.finalize();
        Ok(ContentHash::new(*hash.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let data = b"hello tesseras";
        let h1 = Blake3Hasher::hash(data);
        let h2 = Blake3Hasher::hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_different_input_different_output() {
        let h1 = Blake3Hasher::hash(b"hello");
        let h2 = Blake3Hasher::hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_reader_matches_hash() {
        let data = b"hello tesseras";
        let h1 = Blake3Hasher::hash(data);
        let mut cursor = std::io::Cursor::new(data);
        let h2 = Blake3Hasher::hash_reader(&mut cursor).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_empty_input() {
        let h = Blake3Hasher::hash(b"");
        // BLAKE3 of empty input is a known value
        assert_ne!(h, ContentHash::new([0; 32]));
    }
}
