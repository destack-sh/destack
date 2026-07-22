use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{RegisterRange, TypeId};

/// One tensor value consumed by an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TensorOperand {
    /// The register words containing the tensor value.
    pub registers: RegisterRange,
    /// The type that selects the tensor layout.
    pub ty: TypeId,
}

impl TensorOperand {
    /// The encoded byte length of one tensor operand.
    pub(crate) const BYTE_LEN: usize = size_of::<u16>() * 2 + size_of::<u32>();

    /// Create one tensor operand.
    pub const fn new(registers: RegisterRange, ty: TypeId) -> Self {
        Self { registers, ty }
    }
}

/// One tensor operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TensorOperation {
    /// Apply one elementwise scalar operation.
    Element = 0,
    /// Compare tensor elements.
    Compare = 1,
    /// Select tensor elements.
    Select = 2,
    /// Transpose tensor axes.
    Transpose = 3,
    /// Reshape one tensor.
    Reshape = 4,
    /// Broadcast one tensor.
    Broadcast = 5,
    /// Slice one tensor.
    Slice = 6,
    /// Pad one tensor.
    Pad = 7,
    /// Concatenate tensors.
    Concat = 8,
    /// Fill one tensor with a scalar.
    Splat = 9,
    /// Convert tensor elements.
    Convert = 10,
    /// Reinterpret one tensor without changing its contents.
    Bitcast = 11,
    /// Reduce tensor elements.
    Reduce = 12,
    /// Reduce tensor elements to indices.
    IndexReduce = 13,
    /// Contract paired tensor axes.
    Contract = 14,
    /// Gather tensor elements.
    Gather = 15,
    /// Scatter tensor elements.
    Scatter = 16,
    /// Load one scalar from a tensor view.
    Load = 17,
    /// Extract one scalar from a tensor.
    Extract = 18,
    /// Store one scalar into a tensor view.
    Store = 19,
    /// Fill one tensor view.
    Fill = 20,
    /// Copy one tensor into a tensor view.
    Copy = 21,
    /// Create one tensor view.
    View = 22,
    /// Compute one tensor convolution.
    Convolution = 23,
}

impl TensorOperation {
    /// Return the tensor operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "compare" => Some(Self::Compare),
            "select" => Some(Self::Select),
            "transpose" => Some(Self::Transpose),
            "reshape" => Some(Self::Reshape),
            "broadcast" => Some(Self::Broadcast),
            "slice" => Some(Self::Slice),
            "pad" => Some(Self::Pad),
            "concat" => Some(Self::Concat),
            "splat" => Some(Self::Splat),
            "convert" => Some(Self::Convert),
            "bitcast" => Some(Self::Bitcast),
            "reduce" => Some(Self::Reduce),
            "indexReduce" => Some(Self::IndexReduce),
            "contract" => Some(Self::Contract),
            "gather" => Some(Self::Gather),
            "scatter" => Some(Self::Scatter),
            "load" => Some(Self::Load),
            "extract" => Some(Self::Extract),
            "store" => Some(Self::Store),
            "fill" => Some(Self::Fill),
            "copy" => Some(Self::Copy),
            "view" => Some(Self::View),
            "convolution" => Some(Self::Convolution),
            _ => None,
        }
    }

    /// Decode one stable tensor operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Element),
            1 => Some(Self::Compare),
            2 => Some(Self::Select),
            3 => Some(Self::Transpose),
            4 => Some(Self::Reshape),
            5 => Some(Self::Broadcast),
            6 => Some(Self::Slice),
            7 => Some(Self::Pad),
            8 => Some(Self::Concat),
            9 => Some(Self::Splat),
            10 => Some(Self::Convert),
            11 => Some(Self::Bitcast),
            12 => Some(Self::Reduce),
            13 => Some(Self::IndexReduce),
            14 => Some(Self::Contract),
            15 => Some(Self::Gather),
            16 => Some(Self::Scatter),
            17 => Some(Self::Load),
            18 => Some(Self::Extract),
            19 => Some(Self::Store),
            20 => Some(Self::Fill),
            21 => Some(Self::Copy),
            22 => Some(Self::View),
            23 => Some(Self::Convolution),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Element => "element",
            Self::Compare => "compare",
            Self::Select => "select",
            Self::Transpose => "transpose",
            Self::Reshape => "reshape",
            Self::Broadcast => "broadcast",
            Self::Slice => "slice",
            Self::Pad => "pad",
            Self::Concat => "concat",
            Self::Splat => "splat",
            Self::Convert => "convert",
            Self::Bitcast => "bitcast",
            Self::Reduce => "reduce",
            Self::IndexReduce => "indexReduce",
            Self::Contract => "contract",
            Self::Gather => "gather",
            Self::Scatter => "scatter",
            Self::Load => "load",
            Self::Extract => "extract",
            Self::Store => "store",
            Self::Fill => "fill",
            Self::Copy => "copy",
            Self::View => "view",
            Self::Convolution => "convolution",
        }
    }
}

/// One tensor index reduction operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum IndexReduceOperation {
    /// Select the index of the minimum value.
    Minimum = 0,
    /// Select the index of the maximum value.
    Maximum = 1,
}

impl IndexReduceOperation {
    /// Return the index reduction with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "min" => Some(Self::Minimum),
            "max" => Some(Self::Maximum),
            _ => None,
        }
    }

    /// Decode one stable index reduction operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Minimum),
            1 => Some(Self::Maximum),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimum => "min",
            Self::Maximum => "max",
        }
    }
}

/// Tie breaking for one tensor index reduction.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TieBreak {
    /// Select the first matching index.
    First = 0,
    /// Select the last matching index.
    Last = 1,
}

impl TieBreak {
    /// Return the tie break with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "first" => Some(Self::First),
            "last" => Some(Self::Last),
            _ => None,
        }
    }

    /// Decode one stable tie-breaking code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::First),
            1 => Some(Self::Last),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Last => "last",
        }
    }
}

/// One tensor scatter update operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScatterOperation {
    /// Replace the destination value.
    Replace = 0,
    /// Add to the destination value.
    Add = 1,
    /// Multiply the destination value.
    Multiply = 2,
    /// Select the minimum value.
    Minimum = 3,
    /// Select the maximum value.
    Maximum = 4,
    /// Apply bitwise conjunction.
    And = 5,
    /// Apply bitwise disjunction.
    Or = 6,
    /// Apply bitwise exclusive disjunction.
    Xor = 7,
}

impl ScatterOperation {
    /// Return the scatter operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "replace" => Some(Self::Replace),
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

    /// Decode one stable scatter operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Replace),
            1 => Some(Self::Add),
            2 => Some(Self::Multiply),
            3 => Some(Self::Minimum),
            4 => Some(Self::Maximum),
            5 => Some(Self::And),
            6 => Some(Self::Or),
            7 => Some(Self::Xor),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Add => "add",
            Self::Multiply => "mul",
            Self::Minimum => "min",
            Self::Maximum => "max",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
        }
    }
}
