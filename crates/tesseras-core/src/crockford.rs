//! Crockford Base32 encoding/decoding (zero dependencies).
//!
//! Alphabet: `0123456789ABCDEFGHJKMNPQRSTVWXYZ`
//! - No I, L, O, U (avoids confusion with 1, 1, 0, and profanity)
//! - Case-insensitive decode: O→0, I/L→1

const ENCODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Decode table: ASCII byte → 5-bit value, or 0xFF for invalid.
pub const DECODE_TABLE: [u8; 128] = {
    let mut t = [0xFFu8; 128];
    // Digits
    t[b'0' as usize] = 0;
    t[b'1' as usize] = 1;
    t[b'2' as usize] = 2;
    t[b'3' as usize] = 3;
    t[b'4' as usize] = 4;
    t[b'5' as usize] = 5;
    t[b'6' as usize] = 6;
    t[b'7' as usize] = 7;
    t[b'8' as usize] = 8;
    t[b'9' as usize] = 9;
    // Letters (uppercase)
    t[b'A' as usize] = 10;
    t[b'B' as usize] = 11;
    t[b'C' as usize] = 12;
    t[b'D' as usize] = 13;
    t[b'E' as usize] = 14;
    t[b'F' as usize] = 15;
    t[b'G' as usize] = 16;
    t[b'H' as usize] = 17;
    // no I
    t[b'J' as usize] = 18;
    t[b'K' as usize] = 19;
    // no L
    t[b'M' as usize] = 20;
    t[b'N' as usize] = 21;
    // no O
    t[b'P' as usize] = 22;
    t[b'Q' as usize] = 23;
    t[b'R' as usize] = 24;
    t[b'S' as usize] = 25;
    t[b'T' as usize] = 26;
    // no U
    t[b'V' as usize] = 27;
    t[b'W' as usize] = 28;
    t[b'X' as usize] = 29;
    t[b'Y' as usize] = 30;
    t[b'Z' as usize] = 31;
    // Letters (lowercase)
    t[b'a' as usize] = 10;
    t[b'b' as usize] = 11;
    t[b'c' as usize] = 12;
    t[b'd' as usize] = 13;
    t[b'e' as usize] = 14;
    t[b'f' as usize] = 15;
    t[b'g' as usize] = 16;
    t[b'h' as usize] = 17;
    t[b'j' as usize] = 18;
    t[b'k' as usize] = 19;
    t[b'm' as usize] = 20;
    t[b'n' as usize] = 21;
    t[b'p' as usize] = 22;
    t[b'q' as usize] = 23;
    t[b'r' as usize] = 24;
    t[b's' as usize] = 25;
    t[b't' as usize] = 26;
    t[b'v' as usize] = 27;
    t[b'w' as usize] = 28;
    t[b'x' as usize] = 29;
    t[b'y' as usize] = 30;
    t[b'z' as usize] = 31;
    // Confusable mappings
    t[b'O' as usize] = 0; // O → 0
    t[b'o' as usize] = 0;
    t[b'I' as usize] = 1; // I → 1
    t[b'i' as usize] = 1;
    t[b'L' as usize] = 1; // L → 1
    t[b'l' as usize] = 1;
    t
};

/// Encode bytes to Crockford Base32 (uppercase, no padding).
/// 32 bytes → 52 chars.
pub fn encode(bytes: &[u8]) -> String {
    let bit_len = bytes.len() * 8;
    let out_len = bit_len.div_ceil(5);
    let mut out = String::with_capacity(out_len);

    let mut buffer: u64 = 0;
    let mut bits_in_buffer = 0;

    for &byte in bytes {
        buffer = (buffer << 8) | u64::from(byte);
        bits_in_buffer += 8;
        while bits_in_buffer >= 5 {
            bits_in_buffer -= 5;
            let idx = ((buffer >> bits_in_buffer) & 0x1F) as usize;
            out.push(ENCODE_ALPHABET[idx] as char);
        }
    }
    // Emit remaining bits (left-padded with zeros)
    if bits_in_buffer > 0 {
        let idx = ((buffer << (5 - bits_in_buffer)) & 0x1F) as usize;
        out.push(ENCODE_ALPHABET[idx] as char);
    }

    out
}

/// Decode a full Crockford Base32 string back to bytes.
/// Returns `None` on invalid characters or if `U`/`u` is present.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    let mut buffer: u64 = 0;
    let mut bits_in_buffer = 0;
    let mut out = Vec::new();

    for &b in s.as_bytes() {
        if b == b'U' || b == b'u' {
            return None;
        }
        if b >= 128 {
            return None;
        }
        let val = DECODE_TABLE[b as usize];
        if val == 0xFF {
            return None;
        }
        buffer = (buffer << 5) | u64::from(val);
        bits_in_buffer += 5;
        if bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            out.push((buffer >> bits_in_buffer) as u8);
        }
    }
    Some(out)
}

/// Normalize input: uppercase, map O→0, I/L→1.
/// Returns `None` if any character is invalid for Crockford.
pub fn normalize(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b == b'U' || b == b'u' {
            return None;
        }
        if b >= 128 {
            return None;
        }
        let val = DECODE_TABLE[b as usize];
        if val == 0xFF {
            return None;
        }
        out.push(ENCODE_ALPHABET[val as usize] as char);
    }
    Some(out)
}

/// Convert a base32 prefix to the longest deterministic hex prefix.
/// N base32 chars → floor(N*5/8) full bytes → those bytes as hex.
pub fn prefix_to_hex_prefix(base32_prefix: &str) -> Option<String> {
    let normalized = normalize(base32_prefix)?;
    let decoded = decode(&normalized)?;
    // floor(N*5/8) full bytes are deterministic
    let full_bytes = (base32_prefix.len() * 5) / 8;
    let hex: String = decoded[..full_bytes]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Some(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip_32_bytes() {
        let bytes = [0xABu8; 32];
        let encoded = encode(&bytes);
        assert_eq!(encoded.len(), 52);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(&decoded[..32], &bytes[..]);
    }

    #[test]
    fn encode_known_value() {
        // 0x00 → "00000..."
        let bytes = [0u8; 32];
        let encoded = encode(&bytes);
        assert!(encoded.chars().all(|c| c == '0'));
        assert_eq!(encoded.len(), 52);
    }

    #[test]
    fn case_insensitive_decode() {
        let bytes = [0x42u8; 32];
        let upper = encode(&bytes);
        let lower = upper.to_lowercase();
        let dec_upper = decode(&upper).unwrap();
        let dec_lower = decode(&lower).unwrap();
        assert_eq!(dec_upper, dec_lower);
    }

    #[test]
    fn confusable_o_maps_to_zero() {
        let norm = normalize("O").unwrap();
        assert_eq!(norm, "0");
    }

    #[test]
    fn confusable_i_l_map_to_one() {
        assert_eq!(normalize("I").unwrap(), "1");
        assert_eq!(normalize("L").unwrap(), "1");
        assert_eq!(normalize("l").unwrap(), "1");
        assert_eq!(normalize("i").unwrap(), "1");
    }

    #[test]
    fn reject_u() {
        assert!(normalize("U").is_none());
        assert!(normalize("u").is_none());
        assert!(decode("ABCDU").is_none());
    }

    #[test]
    fn reject_invalid_chars() {
        assert!(normalize("!").is_none());
        assert!(normalize(" ").is_none());
        assert!(decode("ABC DEF").is_none());
    }

    #[test]
    fn prefix_to_hex_prefix_basic() {
        // 8 base32 chars = 40 bits = 5 full bytes
        let mut bytes = [0u8; 32];
        bytes[0] = 0x01;
        bytes[1] = 0x23;
        bytes[2] = 0x45;
        bytes[3] = 0x67;
        bytes[4] = 0x89;
        let encoded = encode(&bytes);
        let prefix8 = &encoded[..8];
        let hex_prefix = prefix_to_hex_prefix(prefix8).unwrap();
        assert_eq!(hex_prefix.len(), 10); // 5 bytes = 10 hex chars
        // The hex prefix must match the original bytes
        let original_hex: String = bytes[..5].iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex_prefix, original_hex);
    }

    #[test]
    fn normalize_mixed_case() {
        let n = normalize("0aGh3z").unwrap();
        assert_eq!(n, "0AGH3Z");
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn roundtrip_32_bytes(bytes in proptest::array::uniform32(any::<u8>())) {
                let encoded = encode(&bytes);
                prop_assert_eq!(encoded.len(), 52);
                let decoded = decode(&encoded).unwrap();
                prop_assert_eq!(&decoded[..32], &bytes[..]);
            }
        }
    }
}
