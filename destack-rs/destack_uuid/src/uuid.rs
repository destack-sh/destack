//! Universally Unique Identifier (UUID) as a [u8; 16] wrapper.

use core::fmt;
use core::str::FromStr;

use crate::format::format_uuid;
use crate::parse::{UuidParseError, parse_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A Universally Unique Identifier (UUID) is a 128-bit identifier.
///
/// Store as `[u8; 16]` in big-endian byte order.
/// No strict RFC compliance guarantees.
pub struct Uuid(pub [u8; 16]);

impl Uuid {
    /// Create from raw `u128`.
    #[inline]
    pub const fn from_u128(value: u128) -> Self {
        Uuid(value.to_be_bytes())
    }

    /// Return raw `u128` value.
    #[inline]
    pub const fn as_u128(self) -> u128 {
        u128::from_be_bytes(self.0)
    }

    /// Return bytes in big-endian order.
    #[inline]
    pub const fn to_bytes_be(self) -> [u8; 16] {
        self.0
    }

    /// Construct from big-endian bytes.
    #[inline]
    pub const fn from_bytes_be(bytes: [u8; 16]) -> Self {
        Uuid(bytes)
    }

    /// Parse a UUID from a string. Dashes are optional. Case-insensitive.
    #[inline]
    pub fn parse_str(s: &str) -> Result<Self, UuidParseError> {
        parse_uuid(s)
    }

    /// Format as hyphenated lowercase string into a stack-allocated buffer.
    /// Returns a `[u8; 36]` containing ASCII characters.
    #[inline]
    pub fn to_hyphenated_lower_bytes(self) -> [u8; 36] {
        format_uuid(self.0)
    }

    /// Convert to an owned hyphenated lowercase `String`.
    #[inline]
    pub fn to_hyphenated_lower_string(self) -> String {
        let buf = self.to_hyphenated_lower_bytes();
        // safe: bytes are ASCII [0-9a-f-]
        unsafe { String::from_utf8_unchecked(buf.to_vec()) }
    }
}

impl fmt::Display for Uuid {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hyphenated_lower_string())
    }
}

impl FromStr for Uuid {
    type Err = UuidParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl From<u128> for Uuid {
    #[inline]
    fn from(value: u128) -> Self {
        Uuid::from_u128(value)
    }
}

impl From<Uuid> for u128 {
    #[inline]
    fn from(value: Uuid) -> Self {
        value.as_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    /// Parse accepts hyphenated and simple uppercase/lowercase.
    fn parse_variants() {
        let s1 = "00112233-4455-6677-8899-aabbccddeeff";
        let s2 = "00112233445566778899AABBCCDDEEFF";
        let a: Uuid = s1.parse().unwrap();
        let b: Uuid = s2.parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.as_u128(), 0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
    }

    #[test]
    /// Macro constructs at compile time.
    fn macro_uuid() {
        let u = crate::uuid!("00112233-4455-6677-8899-aabbccddeeff");
        assert_eq!(u.as_u128(), 0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
        assert_eq!(u.to_string(), "00112233-4455-6677-8899-aabbccddeeff");
    }

    #[quickcheck]
    /// Roundtrip: format then parse yields identical value.
    fn roundtrip(v: u128) -> bool {
        let u = Uuid::from_u128(v);
        let s = u.to_string();
        let p: Uuid = s.parse().unwrap();
        p == u
    }
}
