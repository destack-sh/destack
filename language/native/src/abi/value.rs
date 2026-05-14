use std::error::Error;
use std::fmt;

use destack_engine::{Value, ValueType};
use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

/// Native ABI value tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeValueTag {
    /// The void value tag.
    Void = 0,
    /// The boolean value tag.
    Bool = 1,
    /// The signed integer value tag.
    Int = 2,
    /// The unsigned integer value tag.
    UInt = 3,
    /// The 32-bit float value tag.
    Float32 = 4,
    /// The 64-bit float value tag.
    Float64 = 5,
    /// The character value tag.
    Char = 6,
    /// The local heap reference value tag.
    HeapReference = 7,
    /// The shared heap reference value tag.
    SharedHeapReference = 8,
    /// The local raw pointer value tag.
    RawPointer = 9,
    /// The shared raw pointer value tag.
    SharedRawPointer = 10,
}

/// Native ABI value passed through entrypoint calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NativeValue {
    /// The value tag.
    pub tag: u32,
    /// The integer width in bits.
    pub width: u16,
    /// Reserved value bits.
    pub reserved: u16,
    /// The low payload bits.
    pub low: u64,
    /// The high payload bits.
    pub high: u64,
}

impl NativeValue {
    /// The void ABI value.
    pub const VOID: Self = Self::new(NativeValueTag::Void, 0, 0, 0);

    /// Create one native ABI value.
    pub const fn new(tag: NativeValueTag, width: u16, low: u64, high: u64) -> Self {
        Self {
            tag: tag as u32,
            width,
            reserved: 0,
            low,
            high,
        }
    }

    /// Encode one engine value.
    pub fn from_engine(value: &Value) -> Self {
        match value {
            Value::Void => Self::VOID,
            Value::Bool(value) => Self::new(NativeValueTag::Bool, 1, u64::from(*value), 0),
            Value::Int { value, width } => {
                let bits = *value as u128;
                Self::from_wide(NativeValueTag::Int, *width, bits)
            }
            Value::UInt { value, width } => Self::from_wide(NativeValueTag::UInt, *width, *value),
            Value::Float32 { bits } => Self::new(NativeValueTag::Float32, 32, u64::from(*bits), 0),
            Value::Float64 { bits } => Self::new(NativeValueTag::Float64, 64, *bits, 0),
            Value::Char(value) => Self::new(NativeValueTag::Char, 32, *value as u64, 0),
            Value::HeapReference(reference) => Self::new(
                NativeValueTag::HeapReference,
                usize::BITS as u16,
                reference.bits() as u64,
                0,
            ),
            Value::SharedHeapReference(reference) => Self::new(
                NativeValueTag::SharedHeapReference,
                usize::BITS as u16,
                reference.bits() as u64,
                0,
            ),
            Value::RawPointer(pointer) => Self::new(
                NativeValueTag::RawPointer,
                usize::BITS as u16,
                pointer.bits() as u64,
                0,
            ),
            Value::SharedRawPointer(pointer) => Self::new(
                NativeValueTag::SharedRawPointer,
                usize::BITS as u16,
                pointer.bits() as u64,
                0,
            ),
        }
    }

    /// Decode one engine value.
    pub fn to_engine(self) -> Result<Value, NativeValueError> {
        let tag = NativeValueTag::try_from(self.tag)?;

        match tag {
            NativeValueTag::Void => Ok(Value::Void),
            NativeValueTag::Bool => match self.low {
                0 => Ok(Value::bool(false)),
                1 => Ok(Value::bool(true)),
                value => Err(NativeValueError::InvalidBool { value }),
            },
            NativeValueTag::Int => {
                let width = self.integer_width(ValueType::Int)?;

                Ok(Value::int(self.signed_wide(width), width))
            }
            NativeValueTag::UInt => {
                let width = self.integer_width(ValueType::UInt)?;

                Ok(Value::uint(self.wide(), width))
            }
            NativeValueTag::Float32 => Ok(Value::Float32 {
                bits: self.low as u32,
            }),
            NativeValueTag::Float64 => Ok(Value::Float64 { bits: self.low }),
            NativeValueTag::Char => {
                let value = self.low as u32;
                let Some(value) = char::from_u32(value) else {
                    return Err(NativeValueError::InvalidChar { value });
                };

                Ok(Value::char(value))
            }
            NativeValueTag::HeapReference => {
                let bits = usize::try_from(self.low).map_err(|_| NativeValueError::OutOfRange {
                    value_type: ValueType::HeapReference,
                })?;

                Ok(Value::heap_reference(HeapReference::from_bits(bits)))
            }
            NativeValueTag::SharedHeapReference => {
                let bits = usize::try_from(self.low).map_err(|_| NativeValueError::OutOfRange {
                    value_type: ValueType::SharedHeapReference,
                })?;

                Ok(Value::shared_heap_reference(
                    SharedHeapReference::from_bits(bits),
                ))
            }
            NativeValueTag::RawPointer => {
                let bits = usize::try_from(self.low).map_err(|_| NativeValueError::OutOfRange {
                    value_type: ValueType::RawPointer,
                })?;

                Ok(Value::raw_pointer(RawPointer::from_bits(bits)))
            }
            NativeValueTag::SharedRawPointer => {
                let bits = usize::try_from(self.low).map_err(|_| NativeValueError::OutOfRange {
                    value_type: ValueType::SharedRawPointer,
                })?;

                Ok(Value::shared_raw_pointer(SharedRawPointer::from_bits(bits)))
            }
        }
    }

    /// Encode one wide integer payload.
    const fn from_wide(tag: NativeValueTag, width: u16, bits: u128) -> Self {
        let low = bits as u64;
        let high = (bits >> 64) as u64;

        Self::new(tag, width, low, high)
    }

    /// Decode one wide integer payload.
    const fn wide(self) -> u128 {
        let low = self.low as u128;
        let high = (self.high as u128) << 64;

        high | low
    }

    /// Decode one signed integer payload.
    const fn signed_wide(self, width: u16) -> i128 {
        let bits = self.wide();
        if width >= i128::BITS as u16 {
            bits as i128
        } else {
            let shift = i128::BITS as u16 - width;

            ((bits << shift) as i128) >> shift
        }
    }

    /// Validate one integer width.
    fn integer_width(self, value_type: ValueType) -> Result<u16, NativeValueError> {
        if (1..=128).contains(&self.width) {
            Ok(self.width)
        } else {
            Err(NativeValueError::InvalidWidth {
                value_type,
                width: self.width,
            })
        }
    }
}

/// Native ABI value conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeValueError {
    /// A native value tag is not recognized.
    InvalidTag {
        /// The invalid value tag.
        tag: u32,
    },
    /// A native boolean payload was not canonical.
    InvalidBool {
        /// The invalid boolean payload.
        value: u64,
    },
    /// A native integer width is not valid.
    InvalidWidth {
        /// The value type being decoded.
        value_type: ValueType,
        /// The invalid bit width.
        width: u16,
    },
    /// A native character payload was not a Unicode scalar value.
    InvalidChar {
        /// The invalid character payload.
        value: u32,
    },
    /// A native pointer-sized payload does not fit this target.
    OutOfRange {
        /// The value type being decoded.
        value_type: ValueType,
    },
}

impl fmt::Display for NativeValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTag { tag } => write!(formatter, "invalid native value tag {tag}"),
            Self::InvalidBool { value } => {
                write!(formatter, "invalid native bool value {value}")
            }
            Self::InvalidWidth { value_type, width } => {
                write!(formatter, "invalid native {value_type:?} width {width}")
            }
            Self::InvalidChar { value } => write!(formatter, "invalid native char value {value}"),
            Self::OutOfRange { value_type } => {
                write!(formatter, "native value for {value_type:?} is out of range")
            }
        }
    }
}

impl Error for NativeValueError {}

impl TryFrom<u32> for NativeValueTag {
    type Error = NativeValueError;

    fn try_from(tag: u32) -> Result<Self, Self::Error> {
        match tag {
            0 => Ok(Self::Void),
            1 => Ok(Self::Bool),
            2 => Ok(Self::Int),
            3 => Ok(Self::UInt),
            4 => Ok(Self::Float32),
            5 => Ok(Self::Float64),
            6 => Ok(Self::Char),
            7 => Ok(Self::HeapReference),
            8 => Ok(Self::SharedHeapReference),
            9 => Ok(Self::RawPointer),
            10 => Ok(Self::SharedRawPointer),
            tag => Err(NativeValueError::InvalidTag { tag }),
        }
    }
}
