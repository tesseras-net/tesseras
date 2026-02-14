use tesseras_core::{NodeId, NodeIdentity};

/// Number of leading zero bits required in NodeId for PoW.
pub const POW_DIFFICULTY: u32 = 8;

/// Count leading zero bits in a byte slice.
fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut count = 0;
    for &byte in bytes {
        if byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

/// Generate a NodeIdentity by grinding a nonce until the PoW target is met.
/// NodeId = BLAKE3(public_key || nonce)[..20]
pub fn generate_node_identity(public_key: &[u8; 32]) -> NodeIdentity {
    let mut nonce: u64 = 0;
    loop {
        let node_id = compute_node_id(public_key, nonce);
        if leading_zero_bits(node_id.as_bytes()) >= POW_DIFFICULTY {
            return NodeIdentity {
                node_id,
                public_key: *public_key,
                nonce,
            };
        }
        nonce += 1;
    }
}

/// Compute NodeId = BLAKE3(public_key || nonce)[..20]
pub fn compute_node_id(public_key: &[u8; 32], nonce: u64) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(public_key);
    hasher.update(&nonce.to_le_bytes());
    let hash = hasher.finalize();
    let mut id = [0u8; 20];
    id.copy_from_slice(&hash.as_bytes()[..20]);
    NodeId::new(id)
}

/// Verify that a NodeIdentity has valid proof-of-work.
pub fn verify_pow(identity: &NodeIdentity) -> bool {
    let expected = compute_node_id(&identity.public_key, identity.nonce);
    if expected != identity.node_id {
        return false;
    }
    leading_zero_bits(identity.node_id.as_bytes()) >= POW_DIFFICULTY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_zero_bits_all_zeros() {
        assert_eq!(leading_zero_bits(&[0, 0, 0]), 24);
    }

    #[test]
    fn leading_zero_bits_first_bit_set() {
        assert_eq!(leading_zero_bits(&[0x80]), 0);
    }

    #[test]
    fn leading_zero_bits_low_bit_set() {
        assert_eq!(leading_zero_bits(&[0x01]), 7);
    }

    #[test]
    fn leading_zero_bits_with_leading_zero_byte() {
        assert_eq!(leading_zero_bits(&[0x00, 0x01]), 15);
    }

    #[test]
    fn generate_identity_has_valid_pow() {
        let pubkey = [0xaa; 32];
        let identity = generate_node_identity(&pubkey);
        assert!(verify_pow(&identity));
        assert_eq!(identity.public_key, pubkey);
        assert!(leading_zero_bits(identity.node_id.as_bytes()) >= POW_DIFFICULTY);
    }

    #[test]
    fn verify_rejects_tampered_node_id() {
        let pubkey = [0xbb; 32];
        let mut identity = generate_node_identity(&pubkey);
        // Tamper with node_id
        let mut bad_id = *identity.node_id.as_bytes();
        bad_id[19] ^= 0xff;
        identity.node_id = NodeId::new(bad_id);
        assert!(!verify_pow(&identity));
    }

    #[test]
    fn verify_rejects_wrong_nonce() {
        let pubkey = [0xcc; 32];
        let mut identity = generate_node_identity(&pubkey);
        identity.nonce = identity.nonce.wrapping_add(1);
        assert!(!verify_pow(&identity));
    }

    #[test]
    fn compute_node_id_deterministic() {
        let pubkey = [0xdd; 32];
        let id1 = compute_node_id(&pubkey, 42);
        let id2 = compute_node_id(&pubkey, 42);
        assert_eq!(id1, id2);
    }

    #[test]
    fn compute_node_id_different_nonce_different_id() {
        let pubkey = [0xee; 32];
        let id1 = compute_node_id(&pubkey, 0);
        let id2 = compute_node_id(&pubkey, 1);
        assert_ne!(id1, id2);
    }
}
