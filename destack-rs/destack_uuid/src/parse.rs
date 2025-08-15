//! Parse utilities for `Uuid`.

use std::num::NonZeroU128;

use crate::Uuid;

/// Parse errors for UUID strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UuidParseError {
    /// Input contains a non-hexadecimal character.
    InvalidHex,
    /// After ignoring dashes, the number of hexadecimal digits is not exactly 32.
    InvalidLength,
    /// Zero value.
    Zero,
}

#[inline]
const fn hex_val_const(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("invalid hex character"),
    }
}

/// Parse a UUID string at compile time.
///
/// Accepts either the simple 32-hex form or hyphenated form. Any '-' characters are ignored.
/// Panics at compile time if the string is invalid.
#[allow(dead_code)]
pub(crate) const fn const_parse_uuid(s: &str) -> NonZeroU128 {
    let bytes = s.as_bytes();
    let mut acc: u128 = 0;
    let mut n_hex: usize = 0;
    let mut i: usize = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            i += 1;
            continue;
        }
        let v = hex_val_const(b) as u128;
        if n_hex >= 32 {
            panic!("too many hexadecimal digits");
        }
        acc = (acc << 4) | v;
        n_hex += 1;
        i += 1;
    }
    if n_hex != 32 {
        panic!("uuid requires exactly 32 hexadecimal digits");
    }
    if acc == 0 {
        panic!("zero uuid is invalid")
    }
    NonZeroU128::new(acc).unwrap()
}

#[inline]
fn hex_val_runtime(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse a UUID string at runtime into its raw `u128` representation.
///
/// Accepts either the simple 32-hex form or hyphenated form. Any '-' characters are ignored.
#[inline]
pub(crate) fn parse_uuid(s: &str) -> Result<Uuid, UuidParseError> {
    let bytes = s.as_bytes();
    let mut acc: u128 = 0;
    let mut n_hex: usize = 0;

    for &b in bytes {
        if b == b'-' {
            continue;
        }
        let v = match hex_val_runtime(b) {
            Some(v) => v as u128,
            None => return Err(UuidParseError::InvalidHex),
        };
        if n_hex >= 32 {
            return Err(UuidParseError::InvalidLength);
        }
        acc = (acc << 4) | v;
        n_hex += 1;
    }

    if n_hex != 32 {
        return Err(UuidParseError::InvalidLength);
    }
    if acc == 0 {
        return Err(UuidParseError::Zero);
    }
    let uuid = Uuid(NonZeroU128::new(acc).unwrap());
    Ok(uuid)
}

/// Macro to construct a `Uuid` at compile time from a string literal.
#[macro_export]
macro_rules! uuid {
    ($s:literal) => {{
        const __VAL: NonZeroU128 = $crate::parse::const_parse_uuid($s);
        $crate::Uuid(__VAL)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Parse hyphenated and simple lower/upper hex forms.
    fn parse_basic_forms() {
        let hyph = "00112233-4455-6677-8899-aabbccddeeff";
        let simple = "00112233445566778899aabbccddeeff";
        let upper = "00112233-4455-6677-8899-AABBCCDDEEFF";

        let a = parse_uuid(hyph).unwrap();
        let b = parse_uuid(simple).unwrap();
        let c = parse_uuid(upper).unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.as_u128(), 0x00112233445566778899aabbccddeeffu128);
    }

    #[test]
    /// Invalid characters and lengths are rejected.
    fn parse_invalid() {
        assert!(matches!(parse_uuid(""), Err(UuidParseError::InvalidLength)));
        assert!(matches!(
            parse_uuid("zzzz"),
            Err(UuidParseError::InvalidHex)
        ));
        // too few hex digits
        assert!(matches!(
            parse_uuid("0123"),
            Err(UuidParseError::InvalidLength)
        ));
        // too many hex digits
        assert!(matches!(
            parse_uuid("00112233445566778899aabbccddeeff00"),
            Err(UuidParseError::InvalidLength)
        ));
    }

    #[test]
    /// Compile-time macro parses to the same value.
    fn macro_const_parse() {
        const U: NonZeroU128 = const_parse_uuid("00112233-4455-6677-8899-aabbccddeeff");
        assert_eq!(
            U,
            NonZeroU128::new(0x00112233445566778899aabbccddeeffu128).unwrap()
        );

        let v = parse_uuid("00112233-4455-6677-8899-aabbccddeeff").unwrap();
        assert_eq!(U, NonZeroU128::new(v.as_u128()).unwrap());
    }
}
