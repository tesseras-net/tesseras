use tesseras_crypto::shamir::{HeirShare, ShamirConfig, ShamirSplitter};

#[test]
fn fuzz_shamir_split_recombine() {
    bolero::check!()
        .with_type::<(u8, u8, Vec<u8>)>()
        .for_each(|(threshold, total, secret)| {
            let config = ShamirConfig {
                threshold: *threshold,
                total_shares: *total,
            };
            let owner = [0xAAu8; 32];

            // split should never panic — it should return Ok or Err
            match ShamirSplitter::split(secret, &config, &owner) {
                Ok(shares) => {
                    // If split succeeded, verify basic invariants
                    assert_eq!(shares.len(), *total as usize);
                    for share in &shares {
                        assert!(share.verify_checksum());
                        assert_eq!(share.share_data.len(), secret.len());
                    }

                    // Reconstruct with exactly threshold shares
                    if shares.len() >= *threshold as usize && *threshold > 0 {
                        let subset: Vec<HeirShare> =
                            shares[..*threshold as usize].to_vec();
                        let recovered = ShamirSplitter::reconstruct(&subset, None).unwrap();
                        assert_eq!(recovered, *secret);
                    }
                }
                Err(_) => {
                    // Error is fine for invalid configs (threshold=0, etc)
                }
            }
        });
}

#[test]
fn fuzz_shamir_reconstruct_insufficient() {
    bolero::check!()
        .with_type::<Vec<u8>>()
        .for_each(|secret| {
            if secret.is_empty() {
                return;
            }
            let config = ShamirConfig {
                threshold: 3,
                total_shares: 5,
            };
            let owner = [0xAAu8; 32];
            let shares = ShamirSplitter::split(secret, &config, &owner).unwrap();

            // threshold - 1 shares must fail
            let insufficient: Vec<HeirShare> = shares[..2].to_vec();
            assert!(ShamirSplitter::reconstruct(&insufficient, None).is_err());
        });
}
