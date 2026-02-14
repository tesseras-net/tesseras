use tesseras_core::NodeId;

/// XOR distance between two 160-bit NodeIds.
/// Returns the distance as a [u8; 20] (big-endian, most significant byte first).
pub fn xor_distance(a: &NodeId, b: &NodeId) -> [u8; 20] {
    let mut result = [0u8; 20];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = a.as_bytes()[i] ^ b.as_bytes()[i];
    }
    result
}

/// Determine which k-bucket index a peer belongs to, relative to our node.
/// Returns the index of the most significant differing bit (0-159).
/// Returns None if the two NodeIds are identical.
pub fn bucket_index(local: &NodeId, remote: &NodeId) -> Option<usize> {
    let dist = xor_distance(local, remote);
    for (byte_idx, &byte) in dist.iter().enumerate() {
        if byte != 0 {
            let bit_idx = 7 - byte.leading_zeros() as usize;
            return Some(byte_idx * 8 + (7 - bit_idx));
        }
    }
    None // identical NodeIds
}

/// Compare XOR distances: is `a` closer to `target` than `b`?
pub fn is_closer(target: &NodeId, a: &NodeId, b: &NodeId) -> bool {
    let da = xor_distance(target, a);
    let db = xor_distance(target, b);
    da < db
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(bytes: [u8; 20]) -> NodeId {
        NodeId::new(bytes)
    }

    #[test]
    fn xor_distance_with_self_is_zero() {
        let a = nid([0xff; 20]);
        assert_eq!(xor_distance(&a, &a), [0u8; 20]);
    }

    #[test]
    fn xor_distance_is_symmetric() {
        let a = nid([0x01; 20]);
        let b = nid([0xff; 20]);
        assert_eq!(xor_distance(&a, &b), xor_distance(&b, &a));
    }

    #[test]
    fn bucket_index_identical_is_none() {
        let a = nid([0xab; 20]);
        assert_eq!(bucket_index(&a, &a), None);
    }

    #[test]
    fn bucket_index_one_bit_diff() {
        let a = nid([0x00; 20]);
        let mut b_bytes = [0x00u8; 20];
        b_bytes[19] = 0x01; // lowest bit differs
        let b = nid(b_bytes);
        assert_eq!(bucket_index(&a, &b), Some(159));
    }

    #[test]
    fn bucket_index_highest_bit_diff() {
        let a = nid([0x00; 20]);
        let mut b_bytes = [0x00u8; 20];
        b_bytes[0] = 0x80; // highest bit differs
        let b = nid(b_bytes);
        assert_eq!(bucket_index(&a, &b), Some(0));
    }

    #[test]
    fn is_closer_works() {
        let target = nid([0x00; 20]);
        let mut a_bytes = [0x00u8; 20];
        a_bytes[19] = 0x01; // distance 1
        let a = nid(a_bytes);
        let mut b_bytes = [0x00u8; 20];
        b_bytes[19] = 0x02; // distance 2
        let b = nid(b_bytes);
        assert!(is_closer(&target, &a, &b));
        assert!(!is_closer(&target, &b, &a));
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn xor_symmetry(
                a in proptest::array::uniform20(any::<u8>()),
                b in proptest::array::uniform20(any::<u8>()),
            ) {
                let na = nid(a);
                let nb = nid(b);
                prop_assert_eq!(xor_distance(&na, &nb), xor_distance(&nb, &na));
            }

            #[test]
            fn xor_self_is_zero(a in proptest::array::uniform20(any::<u8>())) {
                let na = nid(a);
                prop_assert_eq!(xor_distance(&na, &na), [0u8; 20]);
            }

            #[test]
            fn xor_triangle_inequality(
                a in proptest::array::uniform20(any::<u8>()),
                b in proptest::array::uniform20(any::<u8>()),
                c in proptest::array::uniform20(any::<u8>()),
            ) {
                let na = nid(a);
                let nb = nid(b);
                let nc = nid(c);
                // XOR satisfies the triangle inequality:
                // For each byte, (a^c) <= (a^b) | (b^c) (bitwise OR upper-bounds XOR)
                let d_ac = xor_distance(&na, &nc);
                let d_ab = xor_distance(&na, &nb);
                let d_bc = xor_distance(&nb, &nc);
                for i in 0..20 {
                    prop_assert!(d_ac[i] <= (d_ab[i] | d_bc[i]));
                }
            }

            #[test]
            fn bucket_index_in_range(
                a in proptest::array::uniform20(any::<u8>()),
                b in proptest::array::uniform20(any::<u8>()),
            ) {
                let na = nid(a);
                let nb = nid(b);
                if let Some(idx) = bucket_index(&na, &nb) {
                    prop_assert!(idx < 160);
                }
            }
        }
    }
}
