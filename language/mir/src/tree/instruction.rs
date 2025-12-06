//! MIR instructions.

use std::fmt;
use std::str::FromStr;

use destack_dir::GlobalSymbolId;
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Constant, Function, Local, LocalNodeId, Node, NodeType, Type, UnaryOperator,
    Value,
};

/// Instructions produce SSA values and perform operations.
/// Each instruction defines at most one value via the `destination` field.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // constants
    /// Load a constant value.
    Constant {
        /// The SSA value to define.
        destination: Value,
        /// The constant value to load.
        value: Constant,
    },

    // arithmetic
    /// Binary operation (e.g., add, subtract, compare).
    Binary {
        /// The SSA value to define with the result.
        destination: Value,
        /// The binary operator to apply.
        operator: BinaryOperator,
        /// The left-hand operand.
        left: Value,
        /// The right-hand operand.
        right: Value,
    },
    /// Unary operation (e.g., negate, not).
    Unary {
        /// The SSA value to define with the result.
        destination: Value,
        /// The unary operator to apply.
        operator: UnaryOperator,
        /// The operand.
        argument: Value,
    },

    // type conversions
    /// Cast between types (bitcast, truncate, extend, etc.).
    Cast {
        /// The SSA value to define with the converted result.
        destination: Value,
        /// The kind of cast to perform.
        kind: CastKind,
        /// The value to cast.
        argument: Value,
        /// The target type to cast to.
        to_type: LocalNodeId<Type>,
    },

    // local variables
    /// Load from a local variable (stack slot).
    LocalGet {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The local variable to load from.
        local: LocalNodeId<Local>,
    },
    /// Store to a local variable (stack slot).
    LocalSet {
        /// The local variable to store to.
        local: LocalNodeId<Local>,
        /// The value to store.
        value: Value,
    },

    // memory (pointers)
    /// Load from a pointer (dereference).
    Load {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The pointer to load from.
        pointer: Value,
    },
    /// Store to a pointer (write through pointer).
    Store {
        /// The pointer to store to.
        pointer: Value,
        /// The value to store.
        value: Value,
    },

    // aggregate operations
    /// Extract a field from a struct or tuple.
    ExtractField {
        /// The SSA value to define with the extracted field.
        destination: Value,
        /// The aggregate value to extract from.
        aggregate: Value,
        /// The zero-based field index.
        index: u32,
    },
    /// Insert a value into a struct or tuple field.
    /// Semantically creates a new aggregate; backends optimize to in-place mutation when possible.
    InsertField {
        /// The SSA value to define with the new aggregate.
        destination: Value,
        /// The original aggregate value.
        aggregate: Value,
        /// The zero-based field index to update.
        index: u32,
        /// The value to insert at the field.
        value: Value,
    },
    /// Extract an element from an array.
    ExtractElement {
        /// The SSA value to define with the extracted element.
        destination: Value,
        /// The array value to extract from.
        array: Value,
        /// The index of the element (runtime value).
        index: Value,
    },
    /// Insert a value into an array element.
    /// Semantically creates a new array; backends optimize to in-place mutation when possible.
    InsertElement {
        /// The SSA value to define with the new array.
        destination: Value,
        /// The original array value.
        array: Value,
        /// The index of the element to update (runtime value).
        index: Value,
        /// The value to insert at the index.
        value: Value,
    },

    // function calls
    /// Call a function directly.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function to call.
        function: FunctionReference,
        /// The arguments to pass.
        arguments: Vec<Value>,
    },
    /// Call through a function pointer (indirect/dynamic dispatch).
    CallIndirect {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function pointer to call.
        callee: Value,
        /// The arguments to pass.
        arguments: Vec<Value>,
    },
}

impl Node for Instruction {
    const TYPE: NodeType = NodeType::Instruction;
}

impl Instruction {
    /// Get the destination value defined by this instruction (if any).
    pub fn destination(&self) -> Option<Value> {
        match self {
            Instruction::Constant { destination, .. } => Some(*destination),
            Instruction::Binary { destination, .. } => Some(*destination),
            Instruction::Unary { destination, .. } => Some(*destination),
            Instruction::Cast { destination, .. } => Some(*destination),
            Instruction::LocalGet { destination, .. } => Some(*destination),
            Instruction::LocalSet { .. } => None,
            Instruction::Load { destination, .. } => Some(*destination),
            Instruction::Store { .. } => None,
            Instruction::ExtractField { destination, .. } => Some(*destination),
            Instruction::InsertField { destination, .. } => Some(*destination),
            Instruction::ExtractElement { destination, .. } => Some(*destination),
            Instruction::InsertElement { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::CallIndirect { destination, .. } => *destination,
        }
    }

    /// Get all values used by this instruction.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        match self {
            Instruction::Constant { .. } => smallvec![],
            Instruction::Binary { left, right, .. } => smallvec![*left, *right],
            Instruction::Unary { argument, .. } => smallvec![*argument],
            Instruction::Cast { argument, .. } => smallvec![*argument],
            Instruction::LocalGet { .. } => smallvec![],
            Instruction::LocalSet { value, .. } => smallvec![*value],
            Instruction::Load { pointer, .. } => smallvec![*pointer],
            Instruction::Store { pointer, value, .. } => smallvec![*pointer, *value],
            Instruction::ExtractField { aggregate, .. } => smallvec![*aggregate],
            Instruction::InsertField {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::ExtractElement { array, index, .. } => smallvec![*array, *index],
            Instruction::InsertElement {
                array,
                index,
                value,
                ..
            } => smallvec![*array, *index, *value],
            Instruction::Call { arguments, .. } => arguments.iter().copied().collect(),
            Instruction::CallIndirect {
                callee, arguments, ..
            } => {
                let mut uses = smallvec![*callee];
                uses.extend(arguments.iter().copied());
                uses
            }
        }
    }
}

/// Reference to a function for call instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionReference {
    /// A function defined in the same module.
    Local(LocalNodeId<Function>),
    /// A function defined in another module (cross-module call).
    Global(GlobalSymbolId),
}

/// Kind of type cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastKind {
    /// Bitcast (reinterpret bits, same size).
    Bitcast,
    /// Truncate integer to smaller width.
    Truncate,
    /// Zero-extend integer to larger width.
    ZeroExtend,
    /// Sign-extend integer to larger width.
    SignExtend,
    /// Convert float to signed integer.
    FloatToSignedInt,
    /// Convert float to unsigned integer.
    FloatToUnsignedInt,
    /// Convert signed integer to float.
    SignedIntToFloat,
    /// Convert unsigned integer to float.
    UnsignedIntToFloat,
    /// Truncate float to smaller width.
    FloatTruncate,
    /// Extend float to larger width.
    FloatExtend,
    /// Pointer to integer.
    PointerToInt,
    /// Integer to pointer.
    IntToPointer,
}

impl CastKind {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            CastKind::Bitcast => "bitcast",
            CastKind::Truncate => "trunc",
            CastKind::ZeroExtend => "uextend",
            CastKind::SignExtend => "sextend",
            CastKind::FloatToSignedInt => "fcvt_to_sint",
            CastKind::FloatToUnsignedInt => "fcvt_to_uint",
            CastKind::SignedIntToFloat => "scvt_to_float",
            CastKind::UnsignedIntToFloat => "ucvt_to_float",
            CastKind::FloatTruncate => "fnarrow",
            CastKind::FloatExtend => "fwiden",
            CastKind::PointerToInt => "ptr_to_int",
            CastKind::IntToPointer => "int_to_ptr",
        }
    }
}

impl fmt::Display for CastKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for CastKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bitcast" => Ok(CastKind::Bitcast),
            "trunc" => Ok(CastKind::Truncate),
            "uextend" => Ok(CastKind::ZeroExtend),
            "sextend" => Ok(CastKind::SignExtend),
            "fcvt_to_sint" => Ok(CastKind::FloatToSignedInt),
            "fcvt_to_uint" => Ok(CastKind::FloatToUnsignedInt),
            "scvt_to_float" => Ok(CastKind::SignedIntToFloat),
            "ucvt_to_float" => Ok(CastKind::UnsignedIntToFloat),
            "fnarrow" => Ok(CastKind::FloatTruncate),
            "fwiden" => Ok(CastKind::FloatExtend),
            "ptr_to_int" => Ok(CastKind::PointerToInt),
            "int_to_ptr" => Ok(CastKind::IntToPointer),
            _ => Err(()),
        }
    }
}
