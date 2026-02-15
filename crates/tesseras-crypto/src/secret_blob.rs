//! Versioned secret blob for heir key recovery.
//!
//! Layout:
//! byte 0:    version (0x01)
//! byte 1:    flags (bit 0 = has_x25519, bit 1 = has_mlkem768)
//! bytes 2-33:  ed25519_secret (32 bytes, always)
//! bytes 34-65: x25519_secret  (if flag bit 0)
//! bytes 66+:   mlkem768_secret (if flag bit 1)

use crate::CryptoError;

/// Parsed key material from a secret blob.
pub struct ParsedSecretBlob {
    pub ed25519_secret: [u8; 32],
    pub x25519_secret: Option<[u8; 32]>,
    pub mlkem768_secret: Option<Vec<u8>>,
}

/// Assemble key material into a versioned blob.
pub fn assemble(
    ed25519_secret: &[u8; 32],
    x25519_secret: Option<&[u8; 32]>,
    mlkem768_secret: Option<&[u8]>,
) -> Vec<u8> {
    let mut flags: u8 = 0;
    if x25519_secret.is_some() {
        flags |= 0x01;
    }
    if mlkem768_secret.is_some() {
        flags |= 0x02;
    }

    let mut blob = Vec::with_capacity(2 + 32 + 32 + mlkem768_secret.map_or(0, |s| s.len()));
    blob.push(0x01); // version
    blob.push(flags);
    blob.extend_from_slice(ed25519_secret);
    if let Some(x) = x25519_secret {
        blob.extend_from_slice(x);
    }
    if let Some(m) = mlkem768_secret {
        blob.extend_from_slice(m);
    }
    blob
}

/// Parse a versioned blob back to key material.
pub fn parse(blob: &[u8]) -> Result<ParsedSecretBlob, CryptoError> {
    if blob.len() < 34 {
        return Err(CryptoError::ShamirReconstructFailed(
            "secret blob too short (need at least 34 bytes)".into(),
        ));
    }
    if blob[0] != 0x01 {
        return Err(CryptoError::ShamirReconstructFailed(format!(
            "unsupported secret blob version: {}",
            blob[0]
        )));
    }

    let flags = blob[1];
    let has_x25519 = flags & 0x01 != 0;
    let has_mlkem768 = flags & 0x02 != 0;

    let ed25519_secret: [u8; 32] = blob[2..34]
        .try_into()
        .map_err(|_| CryptoError::ShamirReconstructFailed("ed25519 slice error".into()))?;

    let mut offset = 34;

    let x25519_secret = if has_x25519 {
        if blob.len() < offset + 32 {
            return Err(CryptoError::ShamirReconstructFailed(
                "secret blob truncated: expected x25519 secret".into(),
            ));
        }
        let x: [u8; 32] = blob[offset..offset + 32]
            .try_into()
            .map_err(|_| CryptoError::ShamirReconstructFailed("x25519 slice error".into()))?;
        offset += 32;
        Some(x)
    } else {
        None
    };

    let mlkem768_secret = if has_mlkem768 {
        if blob.len() <= offset {
            return Err(CryptoError::ShamirReconstructFailed(
                "secret blob truncated: expected mlkem768 secret".into(),
            ));
        }
        Some(blob[offset..].to_vec())
    } else {
        None
    };

    Ok(ParsedSecretBlob {
        ed25519_secret,
        x25519_secret,
        mlkem768_secret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_parse_roundtrip_ed25519_only() {
        let ed_secret = [0x42u8; 32];
        let blob = assemble(&ed_secret, None, None);
        assert_eq!(blob[0], 0x01); // version
        assert_eq!(blob[1], 0x00); // flags: no x25519, no mlkem
        assert_eq!(blob.len(), 34);

        let parsed = parse(&blob).unwrap();
        assert_eq!(parsed.ed25519_secret, ed_secret);
        assert!(parsed.x25519_secret.is_none());
        assert!(parsed.mlkem768_secret.is_none());
    }

    #[test]
    fn assemble_parse_roundtrip_with_x25519() {
        let ed_secret = [0x42u8; 32];
        let x_secret = [0x99u8; 32];
        let blob = assemble(&ed_secret, Some(&x_secret), None);
        assert_eq!(blob[0], 0x01);
        assert_eq!(blob[1], 0x01); // flag bit 0 set
        assert_eq!(blob.len(), 66);

        let parsed = parse(&blob).unwrap();
        assert_eq!(parsed.ed25519_secret, ed_secret);
        assert_eq!(parsed.x25519_secret.unwrap(), x_secret);
        assert!(parsed.mlkem768_secret.is_none());
    }

    #[test]
    fn assemble_parse_roundtrip_full() {
        let ed_secret = [0x42u8; 32];
        let x_secret = [0x99u8; 32];
        let mlkem_secret = vec![0xAAu8; 2400];
        let blob = assemble(&ed_secret, Some(&x_secret), Some(&mlkem_secret));
        assert_eq!(blob[0], 0x01);
        assert_eq!(blob[1], 0x03); // both flag bits set
        assert_eq!(blob.len(), 66 + 2400);

        let parsed = parse(&blob).unwrap();
        assert_eq!(parsed.ed25519_secret, ed_secret);
        assert_eq!(parsed.x25519_secret.unwrap(), x_secret);
        assert_eq!(parsed.mlkem768_secret.unwrap(), mlkem_secret);
    }

    #[test]
    fn parse_rejects_empty_blob() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn parse_rejects_unknown_version() {
        let mut blob = vec![0x02, 0x00]; // version 2
        blob.extend_from_slice(&[0u8; 32]);
        assert!(parse(&blob).is_err());
    }

    #[test]
    fn parse_rejects_truncated_x25519() {
        let mut blob = vec![0x01, 0x01]; // version 1, has_x25519
        blob.extend_from_slice(&[0u8; 32]); // ed25519
        blob.extend_from_slice(&[0u8; 16]); // only 16 bytes of x25519
        assert!(parse(&blob).is_err());
    }
}
