use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::Scalar;

/// One scalar condition required for execution to continue.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScalarCheck {
    /// Require one nonzero scalar.
    Nonzero = 0,
    /// Require one valid shift count.
    Shift = 1,
    /// Require one representable narrowing conversion.
    Narrow = 2,
    /// Require one non-overflowing addition.
    AddOverflow = 3,
    /// Require one non-overflowing subtraction.
    SubtractOverflow = 4,
    /// Require one non-overflowing multiplication.
    MultiplyOverflow = 5,
    /// Require one in-bounds index.
    Bounds = 6,
    /// Require one in-bounds range.
    Range = 7,
}

impl ScalarCheck {
    /// Return the scalar check with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nonzero" => Some(Self::Nonzero),
            "shift" => Some(Self::Shift),
            "narrow" => Some(Self::Narrow),
            "add.overflow" => Some(Self::AddOverflow),
            "sub.overflow" => Some(Self::SubtractOverflow),
            "mul.overflow" => Some(Self::MultiplyOverflow),
            "bounds" => Some(Self::Bounds),
            "range" => Some(Self::Range),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nonzero => "nonzero",
            Self::Shift => "shift",
            Self::Narrow => "narrow",
            Self::AddOverflow => "add.overflow",
            Self::SubtractOverflow => "sub.overflow",
            Self::MultiplyOverflow => "mul.overflow",
            Self::Bounds => "bounds",
            Self::Range => "range",
        }
    }

    /// Decode one stable scalar check code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Nonzero),
            1 => Some(Self::Shift),
            2 => Some(Self::Narrow),
            3 => Some(Self::AddOverflow),
            4 => Some(Self::SubtractOverflow),
            5 => Some(Self::MultiplyOverflow),
            6 => Some(Self::Bounds),
            7 => Some(Self::Range),
            _ => None,
        }
    }

    /// Return whether this check accepts one scalar representation.
    pub const fn supports(self, scalar: Scalar) -> bool {
        match self {
            Self::Nonzero => scalar.is_integer() || scalar.is_float(),
            Self::Shift
            | Self::Narrow
            | Self::AddOverflow
            | Self::SubtractOverflow
            | Self::MultiplyOverflow
            | Self::Bounds
            | Self::Range => scalar.is_integer(),
        }
    }
}

/// One scalar comparison.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Comparison {
    /// Branch when two values are equal.
    Equal = 0,
    /// Branch when two values are unequal.
    NotEqual = 1,
    /// Branch when the left value is less than the right value.
    LessThan = 2,
    /// Branch when the left value is at most the right value.
    LessEqual = 3,
    /// Branch when the left value is greater than the right value.
    GreaterThan = 4,
    /// Branch when the left value is at least the right value.
    GreaterEqual = 5,
}

impl Comparison {
    /// Return the comparison with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "eq" => Some(Self::Equal),
            "ne" => Some(Self::NotEqual),
            "lt" => Some(Self::LessThan),
            "le" => Some(Self::LessEqual),
            "gt" => Some(Self::GreaterThan),
            "ge" => Some(Self::GreaterEqual),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
        }
    }

    /// Decode one stable comparison code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Equal),
            1 => Some(Self::NotEqual),
            2 => Some(Self::LessThan),
            3 => Some(Self::LessEqual),
            4 => Some(Self::GreaterThan),
            5 => Some(Self::GreaterEqual),
            _ => None,
        }
    }

    /// Return whether this branch accepts one scalar representation.
    pub const fn supports(self, scalar: Scalar) -> bool {
        match self {
            Self::Equal
            | Self::NotEqual
            | Self::LessThan
            | Self::LessEqual
            | Self::GreaterThan
            | Self::GreaterEqual => scalar.is_integer() || scalar.is_float(),
        }
    }
}

/// One terminal trap reason.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Trap {
    /// Explicit program abort.
    Abort = 0,
    /// Bounds check failure.
    Bounds = 1,
    /// Null check failure.
    Null = 2,
    /// Integer overflow.
    Overflow = 3,
    /// Invalid arithmetic.
    Arithmetic = 4,
    /// Failed type check.
    Type = 5,
}

impl Trap {
    /// Return the trap with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "abort" => Some(Self::Abort),
            "bounds" => Some(Self::Bounds),
            "null" => Some(Self::Null),
            "overflow" => Some(Self::Overflow),
            "arithmetic" => Some(Self::Arithmetic),
            "type" => Some(Self::Type),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::Bounds => "bounds",
            Self::Null => "null",
            Self::Overflow => "overflow",
            Self::Arithmetic => "arithmetic",
            Self::Type => "type",
        }
    }

    /// Decode one stable trap code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Abort),
            1 => Some(Self::Bounds),
            2 => Some(Self::Null),
            3 => Some(Self::Overflow),
            4 => Some(Self::Arithmetic),
            5 => Some(Self::Type),
            _ => None,
        }
    }
}
