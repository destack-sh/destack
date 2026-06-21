use std::fmt;

use serde::{de, ser};

/// Serialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Custom Serde error message.
    Message(String),
    /// Encoder output length overflowed usize.
    LengthOverflow,
    /// Encoder output did not fit in the caller-owned slice.
    BufferTooSmall,
    /// Sequence length was not known before serialization.
    SequenceLengthRequired,
    /// Map key was serialized before the previous value.
    MapKeyWithoutValue,
    /// Map value was serialized before its key.
    MapValueWithoutKey,
    /// Map key was read without reading its value.
    MapValueNotRead,
    /// Map ended after a key without its value.
    MapEndedWithPendingKey,
    /// Decoder reached the end of input before reading the expected bytes.
    UnexpectedEnd,
    /// Decoder did not consume the whole input.
    TrailingBytes,
    /// Decoder byte range overflowed usize.
    ByteRangeOverflow,
    /// Encoded usize did not fit on this platform.
    UsizeOutOfRange,
    /// Varint used more than the maximum encoded bytes.
    VarintTooLarge,
    /// Varint used a non-canonical byte sequence.
    NonCanonicalVarint,
    /// Boolean tag was not 0 or 1.
    InvalidBool,
    /// Option tag was not 0 or 1.
    InvalidOption,
    /// Character scalar value was invalid.
    InvalidChar,
    /// String bytes were not valid UTF-8.
    InvalidUtf8,
    /// Integer value did not fit the requested target type.
    IntegerOutOfRange(&'static str),
    /// Enum variant index did not fit u32.
    EnumVariantOutOfRange,
    /// Self-describing deserialization was requested.
    SelfDescribingUnsupported,
    /// Ignored-value deserialization was requested.
    IgnoredAnyUnsupported,
}

impl Error {
    /// Create one serialization error.
    pub fn new(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => formatter.write_str(message),
            Self::LengthOverflow => formatter.write_str("encoded length overflow"),
            Self::BufferTooSmall => formatter.write_str("encoded value does not fit output slice"),
            Self::SequenceLengthRequired => formatter.write_str("sequence length is required"),
            Self::MapKeyWithoutValue => formatter.write_str("map key is missing its value"),
            Self::MapValueWithoutKey => formatter.write_str("map value is missing its key"),
            Self::MapValueNotRead => formatter.write_str("map value was not read"),
            Self::MapEndedWithPendingKey => {
                formatter.write_str("map ended with a key missing its value")
            }
            Self::UnexpectedEnd => formatter.write_str("unexpected end of input"),
            Self::TrailingBytes => formatter.write_str("trailing bytes after value"),
            Self::ByteRangeOverflow => formatter.write_str("byte range overflow"),
            Self::UsizeOutOfRange => formatter.write_str("usize value out of range"),
            Self::VarintTooLarge => formatter.write_str("varint is too large"),
            Self::NonCanonicalVarint => formatter.write_str("non-canonical varint"),
            Self::InvalidBool => formatter.write_str("invalid bool tag"),
            Self::InvalidOption => formatter.write_str("invalid option tag"),
            Self::InvalidChar => formatter.write_str("invalid char value"),
            Self::InvalidUtf8 => formatter.write_str("invalid utf-8"),
            Self::IntegerOutOfRange(name) => write!(formatter, "{name} value out of range"),
            Self::EnumVariantOutOfRange => formatter.write_str("enum variant out of range"),
            Self::SelfDescribingUnsupported => {
                formatter.write_str("self describing values are not supported")
            }
            Self::IgnoredAnyUnsupported => formatter.write_str("ignored values are not supported"),
        }
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new(message.to_string())
    }
}

impl de::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new(message.to_string())
    }
}

/// Serialization result.
pub type Result<T> = std::result::Result<T, Error>;
