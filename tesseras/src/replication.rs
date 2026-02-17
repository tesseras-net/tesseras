use reed_solomon_erasure::galois_8::ReedSolomon;

/// A fragment of erasure-coded data.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// Fragment index (0..data_shards+parity_shards).
    pub index: usize,
    /// Fragment data.
    pub data: Vec<u8>,
}

/// Encode data into data+parity fragments using Reed-Solomon.
pub fn encode_fragments(
    data: &[u8],
    data_shards: usize,
    parity_shards: usize,
) -> Result<Vec<Fragment>, ReplicationError> {
    if data.is_empty() {
        return Err(ReplicationError::EmptyData);
    }

    let rs = ReedSolomon::new(data_shards, parity_shards)
        .map_err(|e| ReplicationError::ReedSolomon(format!("{e}")))?;

    // Pad data to be evenly divisible by data_shards
    let shard_size = data.len().div_ceil(data_shards);
    let mut padded = data.to_vec();
    padded.resize(shard_size * data_shards, 0);

    // Split into shards
    let mut shards: Vec<Vec<u8>> = padded.chunks(shard_size).map(|c| c.to_vec()).collect();

    // Add empty parity shards
    for _ in 0..parity_shards {
        shards.push(vec![0u8; shard_size]);
    }

    // Encode parity
    rs.encode(&mut shards)
        .map_err(|e| ReplicationError::ReedSolomon(format!("{e}")))?;

    let fragments = shards
        .into_iter()
        .enumerate()
        .map(|(index, data)| Fragment { index, data })
        .collect();

    Ok(fragments)
}

/// Decode data from fragments using Reed-Solomon.
/// Requires at least `data_shards` fragments to reconstruct.
pub fn decode_fragments(
    fragments: &[Fragment],
    data_shards: usize,
    parity_shards: usize,
    original_size: usize,
) -> Result<Vec<u8>, ReplicationError> {
    if fragments.len() < data_shards {
        return Err(ReplicationError::InsufficientFragments {
            have: fragments.len(),
            need: data_shards,
        });
    }

    let rs = ReedSolomon::new(data_shards, parity_shards)
        .map_err(|e| ReplicationError::ReedSolomon(format!("{e}")))?;

    let total_shards = data_shards + parity_shards;
    let shard_size = fragments[0].data.len();

    // Build shard array with Option (None = missing)
    let mut shards: Vec<Option<Vec<u8>>> = vec![None; total_shards];
    for fragment in fragments {
        if fragment.index < total_shards {
            shards[fragment.index] = Some(fragment.data.clone());
        }
    }

    // Reconstruct missing shards
    rs.reconstruct(&mut shards)
        .map_err(|e| ReplicationError::ReedSolomon(format!("{e}")))?;

    // Concatenate data shards
    let mut result = Vec::with_capacity(shard_size * data_shards);
    for shard in &shards[..data_shards] {
        result.extend_from_slice(shard.as_ref().unwrap());
    }

    // Trim to original size
    result.truncate(original_size);

    Ok(result)
}

#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("empty data")]
    EmptyData,
    #[error("reed-solomon error: {0}")]
    ReedSolomon(String),
    #[error("insufficient fragments: have {have}, need {need}")]
    InsufficientFragments { have: usize, need: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip_small() {
        let data = b"hello tesseras";
        let fragments = encode_fragments(data, 3, 2).unwrap();
        assert_eq!(fragments.len(), 5);

        let decoded = decode_fragments(&fragments, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_decode_roundtrip_1kb() {
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let fragments = encode_fragments(&data, 3, 2).unwrap();

        let decoded = decode_fragments(&fragments, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_decode_roundtrip_1mb() {
        let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let fragments = encode_fragments(&data, 3, 2).unwrap();

        let decoded = decode_fragments(&fragments, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn reconstruct_from_subset_missing_parity() {
        let data = b"reconstruction test data";
        let fragments = encode_fragments(data, 3, 2).unwrap();

        // Use only data shards (drop parity)
        let subset: Vec<Fragment> = fragments.into_iter().take(3).collect();
        let decoded = decode_fragments(&subset, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn reconstruct_from_subset_missing_data() {
        let data = b"reconstruction with missing data shards";
        let fragments = encode_fragments(data, 3, 2).unwrap();

        // Drop first data shard, use shards 1,2,3,4 (indices 1..=4)
        let subset: Vec<Fragment> = fragments.into_iter().skip(1).collect();
        assert_eq!(subset.len(), 4); // more than enough
        let decoded = decode_fragments(&subset, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn fail_with_too_few_shards() {
        let data = b"not enough shards";
        let fragments = encode_fragments(data, 3, 2).unwrap();

        // Only 2 shards, need 3
        let subset: Vec<Fragment> = fragments.into_iter().take(2).collect();
        let result = decode_fragments(&subset, 3, 2, data.len());
        assert!(result.is_err());
    }

    #[test]
    fn encode_single_byte() {
        let data = &[42u8];
        let fragments = encode_fragments(data, 3, 2).unwrap();
        let decoded = decode_fragments(&fragments, 3, 2, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_empty_data_fails() {
        let result = encode_fragments(&[], 3, 2);
        assert!(result.is_err());
    }
}
