use std::fmt;

use destack_core::{FloatFormat, float_to_bits};
use destack_heap::{HeapReference, SharedHeapReference};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::Error;

use super::{TypeId, Word, WordLayout};

/// One value passed into or out of program execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Value {
    /// The void value.
    Void,
    /// One boolean value.
    Bool(bool),
    /// One signed integer value with its width.
    Int {
        /// The integer payload.
        value: i128,
        /// The integer width in bits.
        width: u16,
    },
    /// One unsigned integer value with its width.
    UInt {
        /// The integer payload.
        value: u128,
        /// The integer width in bits.
        width: u16,
    },
    /// One 16-bit IEEE-754 float encoded as raw bits.
    Float16 {
        /// The IEEE-754 payload bits.
        bits: u16,
    },
    /// One 16-bit BF16 float encoded as raw bits.
    Bfloat16 {
        /// The BF16 payload bits.
        bits: u16,
    },
    /// One 32-bit float encoded as raw bits.
    Float32 {
        /// The IEEE-754 payload bits.
        bits: u32,
    },
    /// One 64-bit float encoded as raw bits.
    Float64 {
        /// The IEEE-754 payload bits.
        bits: u64,
    },
    /// One character value.
    Char(char),
    /// One heap reference.
    HeapReference(HeapReference),
    /// One shared heap reference.
    SharedHeapReference(SharedHeapReference),
    /// One native address.
    Address(usize),
    /// One typed value represented by one or more execution words.
    Words {
        /// The concrete Program type.
        ty: TypeId,
        /// The exact execution words.
        words: SmallVec<[Word; 2]>,
    },
}

impl Value {
    /// Create one boolean value.
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create one signed integer value.
    pub const fn int(value: i128, width: u16) -> Self {
        Self::Int { value, width }
    }

    /// Create one signed 8-bit integer value.
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i128, 8)
    }

    /// Create one signed 16-bit integer value.
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i128, 16)
    }

    /// Create one signed 32-bit integer value.
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i128, 32)
    }

    /// Create one signed 64-bit integer value.
    pub const fn int64(value: i64) -> Self {
        Self::int(value as i128, 64)
    }

    /// Create one unsigned integer value.
    pub const fn uint(value: u128, width: u16) -> Self {
        Self::UInt { value, width }
    }

    /// Create one unsigned 8-bit integer value.
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u128, 8)
    }

    /// Create one unsigned 16-bit integer value.
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u128, 16)
    }

    /// Create one unsigned 32-bit integer value.
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u128, 32)
    }

    /// Create one unsigned 64-bit integer value.
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value as u128, 64)
    }

    /// Create one 16-bit IEEE-754 floating point value from bits.
    pub const fn float16_bits(bits: u16) -> Self {
        Self::Float16 { bits }
    }

    /// Create one 16-bit IEEE-754 floating point value.
    pub fn float16(value: f64) -> Self {
        Self::float16_bits(float_to_bits(FloatFormat::Float16, value) as u16)
    }

    /// Create one 16-bit BF16 floating point value from bits.
    pub const fn bfloat16_bits(bits: u16) -> Self {
        Self::Bfloat16 { bits }
    }

    /// Create one 16-bit BF16 floating point value.
    pub fn bfloat16(value: f64) -> Self {
        Self::bfloat16_bits(float_to_bits(FloatFormat::Bfloat16, value) as u16)
    }

    /// Create one 32-bit floating point value.
    pub const fn float32(value: f32) -> Self {
        Self::Float32 {
            bits: value.to_bits(),
        }
    }

    /// Create one 64-bit floating point value.
    pub const fn float64(value: f64) -> Self {
        Self::Float64 {
            bits: value.to_bits(),
        }
    }

    /// Create one character value.
    pub const fn char(value: char) -> Self {
        Self::Char(value)
    }

    /// Create one local heap reference value.
    pub const fn heap_reference(reference: HeapReference) -> Self {
        Self::HeapReference(reference)
    }

    /// Create one shared heap reference value.
    pub const fn shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self::SharedHeapReference(reference)
    }

    /// Create one native address value.
    pub const fn address(address: usize) -> Self {
        Self::Address(address)
    }

    /// Create one typed multiword value.
    pub fn words(ty: TypeId, words: impl IntoIterator<Item = Word>) -> Self {
        Self::Words {
            ty,
            words: words.into_iter().collect(),
        }
    }

    /// Return this value's tag.
    pub const fn tag(&self) -> ValueTag {
        match self {
            Self::Void => ValueTag::Void,
            Self::Bool(_) => ValueTag::Bool,
            Self::Int { .. } => ValueTag::Int,
            Self::UInt { .. } => ValueTag::UInt,
            Self::Float16 { .. } => ValueTag::Float16,
            Self::Bfloat16 { .. } => ValueTag::Bfloat16,
            Self::Float32 { .. } => ValueTag::Float32,
            Self::Float64 { .. } => ValueTag::Float64,
            Self::Char(_) => ValueTag::Char,
            Self::HeapReference(_) => ValueTag::HeapReference,
            Self::SharedHeapReference(_) => ValueTag::SharedHeapReference,
            Self::Address(_) => ValueTag::Address,
            Self::Words { .. } => ValueTag::Words,
        }
    }

    /// Return the concrete type carried by a multiword value.
    pub const fn ty(&self) -> Option<TypeId> {
        match self {
            Self::Words { ty, .. } => Some(*ty),
            _ => None,
        }
    }

    /// Return the raw words carried by a multiword value.
    pub fn as_words(&self) -> Option<&[Word]> {
        match self {
            Self::Words { words, .. } => Some(words),
            _ => None,
        }
    }

    /// Return the raw words carried by a multiword value mutably.
    pub fn as_words_mut(&mut self) -> Option<&mut [Word]> {
        match self {
            Self::Words { words, .. } => Some(words),
            _ => None,
        }
    }

    /// Encode this value through one runtime word layout.
    pub(crate) fn encode(&self, layout: WordLayout) -> crate::Result<Option<Word>> {
        let word = match (layout, self) {
            (WordLayout::Void, Self::Void) => return Ok(None),
            (WordLayout::Boolean, Self::Bool(value)) => Word::boolean(*value),
            (
                WordLayout::Int { width },
                Self::Int {
                    value,
                    width: actual,
                },
            ) => {
                if u16::from(width) != *actual {
                    return Err(Error::IntegerWidthMismatch {
                        expected: width,
                        actual: *actual,
                    });
                }

                Word::int(*value as i64, width)
            }
            (
                WordLayout::Uint { width },
                Self::UInt {
                    value,
                    width: actual,
                },
            ) => {
                if u16::from(width) != *actual {
                    return Err(Error::IntegerWidthMismatch {
                        expected: width,
                        actual: *actual,
                    });
                }

                Word::uint(*value as u64, width)
            }
            (WordLayout::Character, Self::Char(value)) => Word::character(*value),
            (WordLayout::Float16, Self::Float16 { bits })
            | (WordLayout::Bfloat16, Self::Bfloat16 { bits }) => Word::from_bits(u64::from(*bits)),
            (WordLayout::Float32, Self::Float32 { bits }) => Word::from_bits(u64::from(*bits)),
            (WordLayout::Float64, Self::Float64 { bits }) => Word::from_bits(*bits),
            (WordLayout::HeapReference, Self::HeapReference(reference)) => {
                Word::from_bits(reference.bits() as u64)
            }
            (WordLayout::SharedHeapReference, Self::SharedHeapReference(reference)) => {
                Word::from_bits(reference.bits() as u64)
            }
            (
                WordLayout::Address
                | WordLayout::StackPointer
                | WordLayout::FramePointer
                | WordLayout::GlobalAddress
                | WordLayout::FunctionPointer,
                Self::Address(address),
            ) => Word::from_bits(*address as u64),
            (layout, value) => {
                let expected = layout.value_tag();
                let mismatch = ValueMismatch::new(expected, value.tag());

                return Err(Error::value_mismatch(mismatch));
            }
        };

        Ok(Some(word))
    }

    /// Decode one execution word through one runtime word layout.
    pub(crate) fn decode(layout: WordLayout, word: Word) -> crate::Result<Self> {
        let value = match layout {
            WordLayout::Void => Self::Void,
            WordLayout::Boolean => Self::bool(word.as_boolean()),
            WordLayout::Character => {
                let code_point = word.bits() as u32;
                let value =
                    char::from_u32(code_point).ok_or(Error::InvalidCharacter { code_point })?;

                Self::char(value)
            }
            WordLayout::Int { width } => Self::int(word.as_i64() as i128, width.into()),
            WordLayout::Uint { width } => Self::uint(word.as_u64() as u128, width.into()),
            WordLayout::Float16 => Self::float16_bits(word.bits() as u16),
            WordLayout::Bfloat16 => Self::bfloat16_bits(word.bits() as u16),
            WordLayout::Float32 => Self::Float32 {
                bits: word.bits() as u32,
            },
            WordLayout::Float64 => Self::Float64 { bits: word.bits() },
            WordLayout::HeapReference => {
                Self::heap_reference(HeapReference::from_bits(word.bits() as usize))
            }
            WordLayout::SharedHeapReference => {
                Self::shared_heap_reference(SharedHeapReference::from_bits(word.bits() as usize))
            }
            WordLayout::Address
            | WordLayout::StackPointer
            | WordLayout::FramePointer
            | WordLayout::GlobalAddress
            | WordLayout::FunctionPointer => Self::address(word.bits() as usize),
        };

        Ok(value)
    }
}

/// One value discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ValueTag {
    /// The void tag.
    Void,
    /// The boolean tag.
    Bool,
    /// The signed integer tag.
    Int,
    /// The unsigned integer tag.
    UInt,
    /// The 16-bit IEEE-754 floating point tag.
    Float16,
    /// The 16-bit BF16 floating point tag.
    Bfloat16,
    /// The 32-bit floating point tag.
    Float32,
    /// The 64-bit floating point tag.
    Float64,
    /// The character tag.
    Char,
    /// The local heap reference tag.
    HeapReference,
    /// The shared heap reference tag.
    SharedHeapReference,
    /// The native address tag.
    Address,
    /// The typed multiword value tag.
    Words,
}

/// One value tag mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ValueMismatch {
    /// The expected value tag.
    pub expected: ValueTag,
    /// The actual value tag.
    pub actual: ValueTag,
}

impl ValueMismatch {
    /// Create one value tag mismatch.
    pub const fn new(expected: ValueTag, actual: ValueTag) -> Self {
        Self { expected, actual }
    }
}

impl fmt::Display for ValueMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected value {:?}, found {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for ValueMismatch {}

/// One signed integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SignedInt {
    /// The integer payload.
    pub value: i128,
    /// The integer width in bits.
    pub width: u16,
}

/// One unsigned integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct UnsignedInt {
    /// The integer payload.
    pub value: u128,
    /// The integer width in bits.
    pub width: u16,
}

impl TryFrom<&Value> for () {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Void => Ok(()),
            value => Err(ValueMismatch::new(ValueTag::Void, value.tag())),
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(value) => Ok(*value),
            value => Err(ValueMismatch::new(ValueTag::Bool, value.tag())),
        }
    }
}

impl TryFrom<&Value> for SignedInt {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int { value, width } => Ok(Self {
                value: *value,
                width: *width,
            }),
            value => Err(ValueMismatch::new(ValueTag::Int, value.tag())),
        }
    }
}

impl TryFrom<&Value> for UnsignedInt {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::UInt { value, width } => Ok(Self {
                value: *value,
                width: *width,
            }),
            value => Err(ValueMismatch::new(ValueTag::UInt, value.tag())),
        }
    }
}

impl TryFrom<&Value> for f32 {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float32 { bits } => Ok(f32::from_bits(*bits)),
            value => Err(ValueMismatch::new(ValueTag::Float32, value.tag())),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float64 { bits } => Ok(f64::from_bits(*bits)),
            value => Err(ValueMismatch::new(ValueTag::Float64, value.tag())),
        }
    }
}

impl TryFrom<&Value> for char {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Char(value) => Ok(*value),
            value => Err(ValueMismatch::new(ValueTag::Char, value.tag())),
        }
    }
}

impl TryFrom<&Value> for HeapReference {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::HeapReference(reference) => Ok(*reference),
            value => Err(ValueMismatch::new(ValueTag::HeapReference, value.tag())),
        }
    }
}

impl TryFrom<&Value> for SharedHeapReference {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::SharedHeapReference(reference) => Ok(*reference),
            value => Err(ValueMismatch::new(
                ValueTag::SharedHeapReference,
                value.tag(),
            )),
        }
    }
}

impl TryFrom<&Value> for usize {
    type Error = ValueMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Address(address) => Ok(*address),
            value => Err(ValueMismatch::new(ValueTag::Address, value.tag())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves character identity and rejects invalid Unicode code points.
    #[test]
    fn test_encode_decode_character() {
        let value = Value::char('🦀');
        let word = value.encode(WordLayout::Character).unwrap().unwrap();
        let decoded = Value::decode(WordLayout::Character, word).unwrap();

        assert_eq!(decoded, value);

        let invalid = Word::from_bits(0x11_0000);
        let error = Value::decode(WordLayout::Character, invalid).unwrap_err();

        assert_eq!(
            error,
            Error::InvalidCharacter {
                code_point: 0x11_0000,
            }
        );
    }
}
