//! MIR instructions.

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
    Constant { destination: Value, value: Constant },

    // arithmetic
    /// Binary operation.
    Binary {
        destination: Value,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    },
    /// Unary operation.
    Unary {
        destination: Value,
        operator: UnaryOperator,
        argument: Value,
    },

    // type conversions
    /// Cast between types (bitcast, truncate, extend, etc.).
    Cast {
        destination: Value,
        kind: CastKind,
        argument: Value,
        to_type: LocalNodeId<Type>,
    },

    // local variables
    /// Load from a local variable.
    LocalGet {
        destination: Value,
        local: LocalNodeId<Local>,
    },
    /// Store to a local variable.
    LocalSet {
        local: LocalNodeId<Local>,
        value: Value,
    },

    // memory (pointers)
    /// Load from a pointer.
    Load { destination: Value, pointer: Value },
    /// Store to a pointer.
    Store { pointer: Value, value: Value },

    // aggregate operations
    /// Extract a field from a struct/tuple.
    ExtractField {
        destination: Value,
        aggregate: Value,
        index: u32,
    },
    /// Insert a value into a struct/tuple field (creates a new aggregate).
    InsertField {
        destination: Value,
        aggregate: Value,
        index: u32,
        value: Value,
    },
    /// Extract an element from an array.
    ExtractElement {
        destination: Value,
        array: Value,
        index: Value,
    },
    /// Insert a value into an array element (creates a new array).
    InsertElement {
        destination: Value,
        array: Value,
        index: Value,
        value: Value,
    },

    // function calls
    /// Call a function.
    Call {
        destination: Option<Value>,
        function: FunctionReference,
        arguments: Vec<Value>,
    },
    /// Indirect call through a function pointer (dynamic dispatch).
    CallIndirect {
        destination: Option<Value>,
        callee: Value,
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

/// Reference to a function (for calls).
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionReference {
    /// Reference to a function in the same module.
    Local(LocalNodeId<Function>),
    /// Reference to a function in another module.
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
