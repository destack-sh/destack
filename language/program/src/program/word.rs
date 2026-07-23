use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One 64-bit program execution word.
#[repr(C, align(8))]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Word(u64);

impl Word {
    /// The zero word.
    pub const ZERO: Self = Self(0);
    /// The bit width of one word.
    pub const BIT_LEN: u8 = u64::BITS as u8;
    /// The byte width of one word.
    pub const BYTE_LEN: usize = size_of::<Self>();

    /// Create one word from its exact bits.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return this word's exact bits.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// View this word as a boolean.
    #[inline(always)]
    pub const fn as_boolean(self) -> bool {
        self.0 != 0
    }

    /// View this word as a signed integer.
    #[inline(always)]
    pub const fn as_i64(self) -> i64 {
        self.0 as i64
    }

    /// View this word as an unsigned integer.
    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// View this word as a 32-bit float.
    #[inline(always)]
    pub const fn as_f32(self) -> f32 {
        f32::from_bits(self.0 as u32)
    }

    /// View this word as a 64-bit float.
    #[inline(always)]
    pub const fn as_f64(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// View this word as a character.
    #[inline(always)]
    pub fn as_character(self) -> Option<char> {
        char::from_u32(self.0 as u32)
    }

    /// Create one boolean word.
    #[inline(always)]
    pub const fn boolean(value: bool) -> Self {
        Self(value as u64)
    }

    /// Create one signed integer word.
    #[inline(always)]
    pub const fn int(value: i64, width: u8) -> Self {
        Self(Self::truncate_signed(value, width) as u64)
    }

    /// Create one 8-bit signed integer word.
    #[inline(always)]
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i64, 8)
    }

    /// Create one 16-bit signed integer word.
    #[inline(always)]
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i64, 16)
    }

    /// Create one 32-bit signed integer word.
    #[inline(always)]
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i64, 32)
    }

    /// Create one 64-bit signed integer word.
    #[inline(always)]
    pub const fn int64(value: i64) -> Self {
        Self::int(value, 64)
    }

    /// Create one unsigned integer word.
    #[inline(always)]
    pub const fn uint(value: u64, width: u8) -> Self {
        Self(Self::truncate_unsigned(value, width))
    }

    /// Create one 8-bit unsigned integer word.
    #[inline(always)]
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u64, 8)
    }

    /// Create one 16-bit unsigned integer word.
    #[inline(always)]
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u64, 16)
    }

    /// Create one 32-bit unsigned integer word.
    #[inline(always)]
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u64, 32)
    }

    /// Create one 64-bit unsigned integer word.
    #[inline(always)]
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value, 64)
    }

    /// Create one 32-bit floating-point word.
    #[inline(always)]
    pub const fn float32(value: f32) -> Self {
        Self(value.to_bits() as u64)
    }

    /// Create one 64-bit floating-point word.
    #[inline(always)]
    pub const fn float64(value: f64) -> Self {
        Self(value.to_bits())
    }

    /// Create one character word.
    #[inline(always)]
    pub const fn character(value: char) -> Self {
        Self(value as u64)
    }

    /// Return this word as little-endian bytes.
    #[inline(always)]
    pub const fn to_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0.to_le_bytes()
    }

    /// Read one word from exact little-endian bytes.
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes = bytes.try_into().ok()?;

        Some(Self(u64::from_le_bytes(bytes)))
    }

    /// Truncate one unsigned integer to a bit width.
    const fn truncate_unsigned(value: u64, width: u8) -> u64 {
        if width >= u64::BITS as u8 {
            value
        } else {
            value & ((1u64 << width) - 1)
        }
    }

    /// Truncate one signed integer to a bit width.
    const fn truncate_signed(value: i64, width: u8) -> i64 {
        if width >= i64::BITS as u8 {
            return value;
        }

        let mask = (1u64 << width) - 1;
        let value = value as u64 & mask;
        let sign = 1u64 << (width - 1);

        if value & sign == 0 {
            value as i64
        } else {
            (value | !mask) as i64
        }
    }
}

const _: () = assert!(size_of::<Word>() == size_of::<u64>());
