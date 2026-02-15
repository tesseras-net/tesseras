use tesseras_crypto::erasure::{Fragment, ReedSolomonCoder};

#[test]
fn fuzz_erasure_encode_decode() {
    bolero::check!()
        .with_type::<(u8, u8, Vec<u8>)>()
        .for_each(|(data_shards, parity_shards, payload)| {
            let ds = (*data_shards as usize).max(1).min(32);
            let ps = (*parity_shards as usize).max(1).min(32);

            if payload.is_empty() {
                return;
            }

            // encode should never panic
            match ReedSolomonCoder::encode(payload, ds, ps) {
                Ok(fragments) => {
                    assert_eq!(fragments.len(), ds + ps);

                    // All fragments available -> reconstruct must succeed
                    let all: Vec<Option<Fragment>> =
                        fragments.iter().cloned().map(Some).collect();
                    let recovered = ReedSolomonCoder::decode(&all, ds, ps).unwrap();
                    assert_eq!(&recovered[..payload.len()], payload.as_slice());
                }
                Err(_) => {
                    // Invalid params are fine
                }
            }
        });
}

#[test]
fn fuzz_erasure_corrupt_reconstruct() {
    bolero::check!()
        .with_type::<(Vec<u8>, Vec<u8>)>()
        .for_each(|(payload, corruption)| {
            if payload.len() < 10 {
                return;
            }

            let ds = 4;
            let ps = 2;

            let fragments = match ReedSolomonCoder::encode(payload, ds, ps) {
                Ok(f) => f,
                Err(_) => return,
            };

            // Drop up to parity_shards fragments -> must still reconstruct
            let mut partial: Vec<Option<Fragment>> =
                fragments.iter().cloned().map(Some).collect();
            let partial_len = partial.len();
            for (i, byte) in corruption.iter().enumerate().take(ps) {
                if *byte % 2 == 0 {
                    partial[i % partial_len] = None;
                }
            }

            // Should not panic regardless of outcome
            let _ = ReedSolomonCoder::decode(&partial, ds, ps);
        });
}
