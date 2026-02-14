use tesseras_core::replication::{FragmentId, FragmentPlan, FragmentationTier};
use tesseras_core::ContentHash;

use crate::error::ReplicationError;

/// Result of encoding a tessera into fragments.
pub struct EncodeResult {
    pub plan: FragmentPlan,
    /// For Medium/Large tiers: erasure-coded fragments (id, data).
    pub fragments: Vec<(FragmentId, Vec<u8>)>,
    /// For Small tier: raw tessera data (no erasure coding).
    pub raw_data: Vec<u8>,
}

/// Encode a tessera into fragments according to its size tier.
///
/// - Small (< 4 MB): returns raw data for whole-file replication.
/// - Medium/Large: Reed-Solomon encodes into data + parity shards.
pub fn encode_tessera(
    tessera_hash: &ContentHash,
    data: &[u8],
) -> Result<EncodeResult, ReplicationError> {
    let plan = FragmentPlan::new(*tessera_hash, data.len() as u64)?;

    match &plan.tier {
        FragmentationTier::Small { .. } => Ok(EncodeResult {
            plan,
            fragments: Vec::new(),
            raw_data: data.to_vec(),
        }),
        FragmentationTier::Medium {
            data_shards,
            parity_shards,
            ..
        }
        | FragmentationTier::Large {
            data_shards,
            parity_shards,
            ..
        } => {
            let ds = *data_shards as usize;
            let ps = *parity_shards as usize;

            let coded = tesseras_crypto::erasure::ReedSolomonCoder::encode(data, ds, ps)
                .map_err(|e| ReplicationError::Core(tesseras_core::CoreError::InvalidTessera(e.to_string())))?;

            let mut fragments = Vec::with_capacity(coded.len());
            for frag in &coded {
                let checksum = ContentHash::new(blake3::hash(&frag.data).into());
                let id = FragmentId::new(
                    *tessera_hash,
                    frag.index as u16,
                    *data_shards,
                    checksum,
                );
                fragments.push((id, frag.data.clone()));
            }

            Ok(EncodeResult {
                plan,
                fragments,
                raw_data: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::ContentHash;

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    #[test]
    fn encode_small_tier_returns_no_fragments() {
        let data = vec![0xaa; 1000]; // 1KB — well under 4MB
        let result = encode_tessera(&hash(0x01), &data).unwrap();
        assert!(matches!(
            result.plan.tier,
            FragmentationTier::Small { .. }
        ));
        assert!(result.fragments.is_empty());
        assert_eq!(result.raw_data, data);
    }

    #[test]
    fn encode_medium_tier_produces_24_fragments() {
        let data = vec![0xbb; 10 * 1024 * 1024]; // 10 MB
        let result = encode_tessera(&hash(0x02), &data).unwrap();
        assert!(matches!(
            result.plan.tier,
            FragmentationTier::Medium { .. }
        ));
        assert_eq!(result.fragments.len(), 24); // 16 data + 8 parity
        assert!(result.raw_data.is_empty());
    }

    #[test]
    fn encoded_fragments_have_valid_checksums() {
        let data = vec![0xcc; 10 * 1024 * 1024]; // 10 MB
        let result = encode_tessera(&hash(0x03), &data).unwrap();
        for (id, frag_data) in &result.fragments {
            let computed = ContentHash::new(blake3::hash(frag_data).into());
            assert_eq!(id.checksum, computed);
        }
    }

    #[test]
    fn encoded_medium_fragments_can_reconstruct() {
        let data = vec![0xdd; 10 * 1024 * 1024]; // 10 MB
        let result = encode_tessera(&hash(0x04), &data).unwrap();
        let all_fragments: Vec<Option<tesseras_crypto::erasure::Fragment>> = result
            .fragments
            .iter()
            .map(|(id, frag_data)| {
                Some(tesseras_crypto::erasure::Fragment {
                    index: id.index as usize,
                    data: frag_data.clone(),
                })
            })
            .collect();
        let reconstructed =
            tesseras_crypto::erasure::ReedSolomonCoder::decode(&all_fragments, 16, 8).unwrap();
        assert_eq!(&reconstructed[..data.len()], &data[..]);
    }
}
