//! Universally Unique Identifier (UUID) as a u128 wrapper.

use core::fmt;
use core::num::NonZeroU128;
use core::str::FromStr;

use crate::format::format_uuid;
use crate::parse::{UuidParseError, parse_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A Universally Unique Identifier (UUID) is a 128-bit identifier.
///
/// Store as `NonZeroU128` for compactness and speed.
/// No strict RFC compliance guarantees.
/// The zero UUID is invalid (00000000-0000-0000-0000-000000000000).
pub struct Uuid(pub NonZeroU128);

impl Uuid {
    /// Create from raw `u128`.
    #[inline]
    pub const fn from_u128(value: u128) -> Result<Self, UuidParseError> {
        match NonZeroU128::new(value) {
            Some(non_zero) => Ok(Uuid(non_zero)),
            None => Err(UuidParseError::Zero),
        }
    }

    /// Return raw `u128` value.
    #[inline]
    pub const fn as_u128(self) -> u128 {
        self.0.get()
    }

    /// Return bytes in big-endian order.
    #[inline]
    pub const fn to_bytes_be(self) -> [u8; 16] {
        self.0.get().to_be_bytes()
    }

    /// Construct from big-endian bytes.
    #[inline]
    pub const fn from_bytes_be(bytes: [u8; 16]) -> Self {
        let u: u128 = u128::from_be_bytes(bytes);
        if u == 0 {
            panic!("zero UUID")
        }
        Uuid(NonZeroU128::new(u).unwrap())
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
        format_uuid(self.as_u128())
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
    /// Panics if `value` is zero.
    fn from(value: u128) -> Self {
        Uuid(NonZeroU128::new(value).unwrap_or_else(|| panic!("zero UUID")))
    }
}

impl From<Uuid> for u128 {
    #[inline]
    fn from(value: Uuid) -> Self {
        value.0.get()
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
    fn roundtrip(v: NonZeroU128) -> bool {
        let u = Uuid(v);
        let s = u.to_string();
        let p: Uuid = s.parse().unwrap();
        p == u
    }
}
