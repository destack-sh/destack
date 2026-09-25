use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::Scalar;

/// One fixed-width vector register type.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct VectorType {
    /// The scalar lane representation.
    pub scalar: Scalar,
    /// Reserved vector type byte.
    reserved: u8,
    /// The number of lanes.
    pub lane_count: u16,
}

impl VectorType {
    /// Create one fixed-width vector type.
    pub const fn new(scalar: Scalar, lane_count: u16) -> Self {
        Self {
            scalar,
            reserved: 0,
            lane_count,
        }
    }

    /// Parse one canonical opcode representation name.
    pub fn from_name(name: &str) -> Option<Self> {
        let (scalar, lane_count) = name.rsplit_once('x')?;
        let scalar = Scalar::from_name(scalar)?;
        let lane_count = lane_count.parse::<u16>().ok()?;

        (lane_count != 0).then_some(Self::new(scalar, lane_count))
    }

    /// Return the canonical opcode representation name.
    pub fn name(self) -> String {
        format!("{}x{}", self.scalar.name(), self.lane_count)
    }

    /// Return the number of contiguous register words occupied by this vector.
    pub const fn word_count(self) -> u16 {
        let bit_count = self.scalar.bit_width() as u32 * self.lane_count as u32;

        bit_count.div_ceil(u64::BITS) as u16
    }

    /// Return the boolean vector type produced by lane comparisons.
    pub const fn mask(self) -> Self {
        Self::new(Scalar::Boolean, self.lane_count)
    }
}

/// One vector operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum VectorOperation {
    /// Fill every vector lane with one scalar.
    Splat = 0,
    /// Insert one scalar lane.
    Insert = 1,
    /// Extract one scalar lane.
    Extract = 2,
    /// Shuffle lanes from two vectors.
    Shuffle = 3,
    /// Apply one elementwise scalar operation.
    Element = 4,
    /// Compare vector lanes.
    Compare = 5,
    /// Select vector lanes.
    Select = 6,
    /// Reduce vector lanes.
    Reduce = 7,
    /// Convert vector lanes.
    Convert = 8,
    /// Load one vector from an address.
    Load = 9,
    /// Store one vector to an address.
    Store = 10,
}

impl VectorOperation {
    /// Decode one stable vector operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Splat),
            1 => Some(Self::Insert),
            2 => Some(Self::Extract),
            3 => Some(Self::Shuffle),
            4 => Some(Self::Element),
            5 => Some(Self::Compare),
            6 => Some(Self::Select),
            7 => Some(Self::Reduce),
            8 => Some(Self::Convert),
            9 => Some(Self::Load),
            10 => Some(Self::Store),
            _ => None,
        }
    }
}

/// One vector reduction operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ReduceOperation {
    /// Add all selected values.
    Add = 0,
    /// Multiply all selected values.
    Multiply = 1,
    /// Select the minimum value.
    Minimum = 2,
    /// Select the maximum value.
    Maximum = 3,
    /// Apply bitwise conjunction.
    And = 4,
    /// Apply bitwise disjunction.
    Or = 5,
    /// Apply bitwise exclusive disjunction.
    Xor = 6,
}

impl ReduceOperation {
    /// Return the reduction operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "mul" => Some(Self::Multiply),
            "min" => Some(Self::Minimum),
            "max" => Some(Self::Maximum),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "xor" => Some(Self::Xor),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Multiply => "mul",
            Self::Minimum => "min",
            Self::Maximum => "max",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
        }
    }

    /// Decode one stable reduction operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Add),
            1 => Some(Self::Multiply),
            2 => Some(Self::Minimum),
            3 => Some(Self::Maximum),
            4 => Some(Self::And),
            5 => Some(Self::Or),
            6 => Some(Self::Xor),
            _ => None,
        }
    }
}

/// One vector element conversion mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ConvertMode {
    /// Require an exact representable result.
    Exact = 0,
    /// Round to the nearest value, breaking ties toward even.
    RoundTiesEven = 1,
    /// Round toward zero.
    RoundTowardZero = 2,
    /// Round toward negative infinity.
    RoundFloor = 3,
    /// Round toward positive infinity.
    RoundCeil = 4,
    /// Clamp values outside the destination range.
    Saturate = 5,
}

impl ConvertMode {
    /// Return the conversion mode with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "exact" => Some(Self::Exact),
            "roundTiesEven" => Some(Self::RoundTiesEven),
            "roundTowardZero" => Some(Self::RoundTowardZero),
            "roundFloor" => Some(Self::RoundFloor),
            "roundCeil" => Some(Self::RoundCeil),
            "saturate" => Some(Self::Saturate),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::RoundTiesEven => "roundTiesEven",
            Self::RoundTowardZero => "roundTowardZero",
            Self::RoundFloor => "roundFloor",
            Self::RoundCeil => "roundCeil",
            Self::Saturate => "saturate",
        }
    }

    /// Decode one stable conversion mode code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Exact),
            1 => Some(Self::RoundTiesEven),
            2 => Some(Self::RoundTowardZero),
            3 => Some(Self::RoundFloor),
            4 => Some(Self::RoundCeil),
            5 => Some(Self::Saturate),
            _ => None,
        }
    }
}

const _: () = assert!(size_of::<VectorType>() == size_of::<u32>());
