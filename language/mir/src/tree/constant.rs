use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use tspp_core::StringId;

use crate::{FloatType, TypeId};

/// A compile-time constant value in MIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Constant {
    /// The value one template parameter names.
    Parameter(u32),
    /// Null reference constant.
    Null,
    /// Undefined constant at a polymorphic representation, specialized at instantiation.
    Undefined,
    /// A measure of one type's layout.
    Layout {
        /// The measured type.
        ty: TypeId,
        /// The measure taken.
        measure: LayoutMeasure,
    },
    /// The associated const one type answers an interface with, resolved at instantiation.
    Witness {
        /// The type answering the interface.
        receiver: TypeId,
        /// The applied interface declaring the const.
        interface: TypeId,
        /// The associated const name.
        member: StringId,
    },
    /// Boolean constant.
    Boolean {
        /// The boolean value.
        value: bool,
    },
    /// Signed integer constant.
    Int {
        /// The value, sign-extended to 128 bits.
        value: i128,
        /// The bit width of the integer type.
        width: u16,
        /// Whether this represents a signed integer type.
        is_signed: bool,
    },
    /// Unsigned integer constant.
    UInt {
        /// The value, zero-extended to 128 bits.
        value: u128,
        /// The bit width of the integer type.
        width: u16,
    },
    /// Floating point constant.
    Float {
        /// The value stored as raw bits.
        bits: u64,
        /// The concrete float format.
        format: FloatType,
    },
    /// Character constant.
    Char {
        /// The character value.
        value: char,
    },
    /// Uninitialized storage; reads are invalid until initialized.
    Uninit,
    /// Storage with every byte zeroed.
    Zeroed,
}

impl Constant {
    /// Create a null constant.
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a new integer constant.
    pub fn int8(value: i8) -> Self {
        Self::Int {
            value: value as i128,
            width: 8,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int16(value: i16) -> Self {
        Self::Int {
            value: value as i128,
            width: 16,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int32(value: i32) -> Self {
        Self::Int {
            value: value as i128,
            width: 32,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int64(value: i64) -> Self {
        Self::Int {
            value: value as i128,
            width: 64,
            is_signed: true,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint8(value: u8) -> Self {
        Self::UInt {
            value: value as u128,
            width: 8,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint16(value: u16) -> Self {
        Self::UInt {
            value: value as u128,
            width: 16,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint32(value: u32) -> Self {
        Self::UInt {
            value: value as u128,
            width: 32,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint64(value: u64) -> Self {
        Self::UInt {
            value: value as u128,
            width: 64,
        }
    }

    /// Create a new floating point constant.
    pub fn float32(value: f32) -> Self {
        Self::Float {
            bits: value.to_bits() as u64,
            format: FloatType::Float32,
        }
    }

    /// Create a new floating point constant.
    pub fn float64(value: f64) -> Self {
        Self::Float {
            bits: value.to_bits(),
            format: FloatType::Float64,
        }
    }

    /// Create a new boolean constant.
    pub fn boolean(value: bool) -> Self {
        Self::Boolean { value }
    }

    /// Create a new character constant.
    pub fn char(value: char) -> Self {
        Self::Char { value }
    }
}

/// One measure a layout constant takes of its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LayoutMeasure {
    /// The size in bytes.
    Size,
    /// The alignment in bytes.
    Alignment,
    /// The size padded to the alignment.
    Stride,
}

impl LayoutMeasure {
    /// Return the keyword naming this measure.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Size => "size.of",
            Self::Alignment => "align.of",
            Self::Stride => "stride.of",
        }
    }

    /// Return the measure one keyword names.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "size.of" => Some(Self::Size),
            "align.of" => Some(Self::Alignment),
            "stride.of" => Some(Self::Stride),
            _ => None,
        }
    }
}
