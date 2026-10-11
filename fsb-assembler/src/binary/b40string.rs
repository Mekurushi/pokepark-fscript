use crate::error::{AssemblerError, AssemblerResult, BinaryReadError, BinaryReadResult};
const B40_ALPHABET: &[u8; 40] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-/";

fn encode_b40_uint(s: &[u8]) -> AssemblerResult<u32> {
    let mut result = 0u32;

    for &byte in s.iter().take(6) {
        let upper = byte.to_ascii_uppercase();
        let idx = B40_ALPHABET
            .iter()
            .position(|&c| c == upper)
            .ok_or(AssemblerError::InvalidB40Char(upper as char))?;
        let idx =
            u32::try_from(idx).map_err(|_foo| AssemblerError::InvalidB40Char(upper as char))?;
        result = result * 40 + idx;
    }
    for _ in s.len()..6 {
        result *= 40;
    }
    Ok(result)
}
pub fn encode_b40(name: &str) -> AssemblerResult<[u8; 8]> {
    let bytes = name.as_bytes();
    let first = encode_b40_uint(&bytes[..bytes.len().min(6)])?;
    // just silently truncate longer strings TODO: explicitly define behavior
    let second = encode_b40_uint(if bytes.len() > 6 {
        &bytes[6..12.min(bytes.len())]
    } else {
        &[]
    })?;
    let mut out = [0u8; 8];
    out[0..4].copy_from_slice(&first.to_be_bytes());
    out[4..8].copy_from_slice(&second.to_be_bytes());
    Ok(out)
}
fn decode_b40_uint(mut val: u32) -> BinaryReadResult<String> {
    let encoded = val;
    let mut chars = [0u8; 6];
    for i in (0..6).rev() {
        let idx = (val % 40) as usize;
        chars[i] = B40_ALPHABET[idx];
        val /= 40;
    }
    String::from_utf8(chars.to_vec()).map_err(|_| BinaryReadError::InvalidB40 {
        location: "B40 string segment".to_string(),
        value: encoded,
    })
}
pub fn decode_b40(bytes: [u8; 8]) -> BinaryReadResult<String> {
    let a = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let b = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let mut result = decode_b40_uint(a)?;
    result.push_str(&decode_b40_uint(b)?);
    Ok(result.trim_end().to_string())
}
#[allow(unused)]
pub fn validate_b40_chars(name: &str) -> AssemblerResult<()> {
    for byte in name.bytes() {
        let upper = byte.to_ascii_uppercase();
        if !B40_ALPHABET.contains(&upper) {
            return Err(AssemblerError::InvalidB40Char(upper as char));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    use super::*;

    #[test]
    fn test_b40_encode() {
        let encoded = encode_b40("EVAR01ZN01_N").unwrap();
        assert_eq!(encoded, [0x60, 0x7a, 0xed, 0x2a, 0xdf, 0x64, 0x8c, 0x60]);
    }

    #[test]
    fn test_b40_decode() {
        let decoded = decode_b40([0x60, 0x7a, 0xed, 0x2a, 0xdf, 0x64, 0x8c, 0x60]).unwrap();
        assert_eq!(decoded, "EVAR01ZN01_N");
    }

    #[test]
    fn test_b40_roundtrip() {
        let name = "ADD";
        let encoded = encode_b40(name).unwrap();
        let decoded = decode_b40(encoded).unwrap();
        assert_eq!(decoded, name);
    }
    #[test]
    fn test_b40_roundtrip_full_len() {
        let name = "EVAR01ZN01_N";
        let encoded = encode_b40(name).unwrap();
        let decoded = decode_b40(encoded).unwrap();
        assert_eq!(decoded, name);
    }

    #[test]
    fn test_b40_invalid_char() {
        let result = encode_b40("INVALID!");
        assert!(matches!(result, Err(AssemblerError::InvalidB40Char('!'))));
    }

    #[test]
    fn test_validate_b40_chars() {
        let invalid_result = validate_b40_chars("INVALID!");
        assert!(matches!(
            invalid_result,
            Err(AssemblerError::InvalidB40Char('!'))
        ));
        let valid_result = validate_b40_chars("VALID");
        assert!(matches!(valid_result, Ok(..)));
    }
}
