use crate::CryptoError;

#[derive(Debug, Clone)]
pub struct Fragment {
    pub index: usize,
    pub data: Vec<u8>,
}

pub struct ReedSolomonCoder;

impl ReedSolomonCoder {
    pub fn encode(
        data: &[u8],
        data_shards: usize,
        parity_shards: usize,
    ) -> Result<Vec<Fragment>, CryptoError> {
        use reed_solomon_erasure::galois_8::ReedSolomon;
        let r = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| CryptoError::ErasureError(e.to_string()))?;
        let shard_size = (data.len() + data_shards - 1) / data_shards;
        let mut shards: Vec<Vec<u8>> = (0..data_shards)
            .map(|i| {
                let start = i * shard_size;
                let end = ((i + 1) * shard_size).min(data.len());
                let mut shard = vec![0u8; shard_size];
                if start < data.len() {
                    let slice = &data[start..end];
                    shard[..slice.len()].copy_from_slice(slice);
                }
                shard
            })
            .collect();
        // Add empty parity shards
        for _ in 0..parity_shards {
            shards.push(vec![0u8; shard_size]);
        }
        r.encode(&mut shards)
            .map_err(|e| CryptoError::ErasureError(e.to_string()))?;
        Ok(shards
            .into_iter()
            .enumerate()
            .map(|(index, data)| Fragment { index, data })
            .collect())
    }

    pub fn decode(
        fragments: &[Option<Fragment>],
        data_shards: usize,
        parity_shards: usize,
    ) -> Result<Vec<u8>, CryptoError> {
        use reed_solomon_erasure::galois_8::ReedSolomon;
        let r = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| CryptoError::ErasureError(e.to_string()))?;
        let shard_size = fragments
            .iter()
            .find_map(|f| f.as_ref().map(|f| f.data.len()))
            .ok_or_else(|| CryptoError::ErasureError("no fragments provided".into()))?;
        let mut shards: Vec<Option<Vec<u8>>> = fragments
            .iter()
            .map(|f| f.as_ref().map(|f| f.data.clone()))
            .collect();
        r.reconstruct(&mut shards)
            .map_err(|e| CryptoError::ErasureError(e.to_string()))?;
        let mut result = Vec::with_capacity(data_shards * shard_size);
        for shard in &shards[..data_shards] {
            result.extend_from_slice(shard.as_ref().unwrap());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let data = b"hello tesseras erasure coding test data that is long enough";
        let fragments = ReedSolomonCoder::encode(data, 4, 2).unwrap();
        assert_eq!(fragments.len(), 6); // 4 data + 2 parity
        let all: Vec<Option<Fragment>> = fragments.into_iter().map(Some).collect();
        let recovered = ReedSolomonCoder::decode(&all, 4, 2).unwrap();
        assert_eq!(&recovered[..data.len()], data);
    }

    #[test]
    fn decode_with_max_tolerable_loss() {
        let data = b"hello tesseras erasure coding test data that is long enough";
        let fragments = ReedSolomonCoder::encode(data, 4, 2).unwrap();
        // Drop exactly parity_shards (2) fragments — should succeed
        let mut partial: Vec<Option<Fragment>> = fragments.into_iter().map(Some).collect();
        partial[0] = None;
        partial[1] = None;
        let recovered = ReedSolomonCoder::decode(&partial, 4, 2).unwrap();
        assert_eq!(&recovered[..data.len()], data);
    }

    #[test]
    fn decode_fails_with_too_many_lost() {
        let data = b"hello tesseras erasure coding test data that is long enough";
        let fragments = ReedSolomonCoder::encode(data, 4, 2).unwrap();
        // Drop parity_shards + 1 (3) fragments — should fail
        let mut partial: Vec<Option<Fragment>> = fragments.into_iter().map(Some).collect();
        partial[0] = None;
        partial[1] = None;
        partial[2] = None;
        let result = ReedSolomonCoder::decode(&partial, 4, 2);
        assert!(result.is_err());
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn erasure_boundary_roundtrip(
                data in proptest::collection::vec(any::<u8>(), 100..1000),
                data_shards in 2usize..8,
                parity_shards in 1usize..4,
            ) {
                let fragments = ReedSolomonCoder::encode(&data, data_shards, parity_shards).unwrap();
                let total = data_shards + parity_shards;
                prop_assert_eq!(fragments.len(), total);

                // Drop exactly parity_shards (max tolerable) — should succeed
                let mut partial: Vec<Option<Fragment>> = fragments.iter().cloned().map(Some).collect();
                for i in 0..parity_shards {
                    partial[i] = None;
                }
                let recovered = ReedSolomonCoder::decode(&partial, data_shards, parity_shards).unwrap();
                prop_assert_eq!(&recovered[..data.len()], &data[..]);
            }
        }
    }
}
