//! MIR instructions.

use std::fmt;
use std::str::FromStr;

use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Constant, Function, Global, Intrinsic, Local, LocalNodeId, MemoryOrdering,
    Node, NodeType, Type, UnaryOperator, Value,
};

/// Compact representation of an argument slice stored in an external buffer.
///
/// Used by Call, CallIndirect, and Intrinsic instructions to reference arguments.
/// This saves 16 bytes per instruction compared to using `Vec<Value>` inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArgumentSlice {
    /// Start index in the arguments buffer.
    pub start: u32,
    /// Number of arguments.
    pub count: u16,
}

impl ArgumentSlice {
    /// Create a new argument slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Check if the slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the length of the slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Instructions produce SSA values and perform "operations".
/// Each instruction produces at most one value via the `destination` field.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // constants
    /// Load a constant value.
    Const {
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
        /// The cast operator to perform.
        operator: CastOperator,
        /// The value to cast.
        argument: Value,
        /// The target type to cast to.
        to_type: LocalNodeId<Type>,
    },

    // local variables (local.get, local.set)
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

    // global variables (global.addr, global.const)
    /// Get the address of a mutable global variable.
    /// Returns a raw pointer that can be used with Load/Store.
    GlobalAddr {
        /// The SSA value to define with the pointer.
        destination: Value,
        /// The global variable to get the address of.
        global: LocalNodeId<Global>,
    },
    /// Load the value of an immutable global constant.
    /// Returns the constant value directly.
    GlobalConst {
        /// The SSA value to define with the constant value.
        destination: Value,
        /// The global constant to load.
        global: LocalNodeId<Global>,
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

    // drop
    /// Drop a value (drop).
    Drop {
        /// The value to drop.
        value: Value,
    },

    // aggregate operations (field.get, field.addr, field.set, element.get, element.addr, element.set)
    /// Extract a field from an aggregate value (field.get).
    FieldGet {
        /// The SSA value to define with the extracted field.
        destination: Value,
        /// The aggregate value to extract from.
        aggregate: Value,
        /// The zero-based field index.
        index: u32,
    },
    /// Get the address of a field from an aggregate value (field.addr).
    FieldAddr {
        /// The SSA value to define with the field address.
        destination: Value,
        /// The aggregate value to get the field from.
        aggregate: Value,
        /// The zero-based field index.
        index: u32,
    },
    /// Insert a value into a struct or tuple field (field.set).
    FieldSet {
        /// The SSA value to define with the new aggregate.
        destination: Value,
        /// The original aggregate value.
        aggregate: Value,
        /// The zero-based field index to update.
        index: u32,
        /// The value to insert at the field.
        value: Value,
    },
    /// Extract an element from an array aggregate (element.get).
    ElementGet {
        /// The SSA value to define with the extracted element.
        destination: Value,
        /// The array value to extract from.
        array: Value,
        /// The index of the element (runtime value).
        index: Value,
    },
    /// Get the address of an array element from an aggregate (element.addr).
    ElementAddr {
        /// The SSA value to define with the element address.
        destination: Value,
        /// The array value to get the element from.
        array: Value,
        /// The index of the element (runtime value).
        index: Value,
    },
    /// Insert a value into an array element (element.set).
    ElementSet {
        /// The SSA value to define with the new array.
        destination: Value,
        /// The original array value.
        array: Value,
        /// The index of the element to update (runtime value).
        index: Value,
        /// The value to insert at the index.
        value: Value,
    },

    // function calls (call, call.indirect)
    /// Call a function directly.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function to call.
        function: LocalNodeId<Function>,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
    },
    /// Call through a function pointer (call.indirect).
    CallIndirect {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function pointer to call.
        callee: Value,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
    },

    // allocation (managed - runtime tracks memory: managed.alloc, managed.alloc_array)
    /// Allocate a managed (runtime-tracked) struct (managed.alloc).
    /// Returns a `ref<managed T>`.
    ManagedAlloc {
        /// The SSA value to define with the allocated reference.
        destination: Value,
        /// The type of the struct to allocate.
        layout: LocalNodeId<Type>,
    },
    /// Allocate a managed array (managed.alloc_array).
    /// Returns a `ref<managed [T]>`.
    ManagedAllocArray {
        /// The SSA value to define with the allocated reference.
        destination: Value,
        /// The element type of the array.
        element: LocalNodeId<Type>,
        /// The number of elements (runtime value).
        length: Value,
    },

    // allocation (raw - manual memory management: raw.alloc, raw.free)
    /// Allocate raw memory on the heap (raw.alloc).
    /// Returns a `ref<raw T>`. Caller must free with `raw.free`.
    RawAlloc {
        /// The SSA value to define with the allocated pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
    },
    /// Free raw heap memory previously allocated with `raw.alloc` (raw.free).
    RawFree {
        /// The pointer to free.
        pointer: Value,
    },

    // allocation (stack - automatic, scoped to function: stack.alloc)
    /// Allocate on the stack (lives until function returns) (stack.alloc).
    /// Returns a `ref<raw T>`. Cannot free explicitly.
    StackAlloc {
        /// The SSA value to define with the stack pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
    },

    // intrinsics
    /// Call a compiler intrinsic.
    ///
    /// Intrinsics are special operations that:
    /// - Have no function body (handled specially by each backend)
    /// - May have target-specific implementations
    /// - Are used for comptime evaluation, type reflection, and low-level ops
    Intrinsic {
        /// The SSA value to define with the result, if any.
        destination: Option<Value>,
        /// The intrinsic to call.
        intrinsic: Intrinsic,
        /// The arguments to pass.
        arguments: ArgumentSlice,
        /// Memory ordering for atomic operations (None for non-atomic intrinsics).
        ordering: Option<MemoryOrdering>,
    },
}

impl Node for Instruction {
    const TYPE: NodeType = NodeType::Instruction;
}

impl Instruction {
    /// Get the destination value defined by this instruction (if any).
    pub fn destination(&self) -> Option<Value> {
        match self {
            Instruction::Const { destination, .. } => Some(*destination),
            Instruction::Binary { destination, .. } => Some(*destination),
            Instruction::Unary { destination, .. } => Some(*destination),
            Instruction::Cast { destination, .. } => Some(*destination),
            Instruction::LocalGet { destination, .. } => Some(*destination),
            Instruction::LocalSet { .. } => None,
            Instruction::GlobalAddr { destination, .. } => Some(*destination),
            Instruction::GlobalConst { destination, .. } => Some(*destination),
            Instruction::Load { destination, .. } => Some(*destination),
            Instruction::Store { .. } => None,
            Instruction::Drop { .. } => None,
            Instruction::FieldGet { destination, .. } => Some(*destination),
            Instruction::FieldAddr { destination, .. } => Some(*destination),
            Instruction::FieldSet { destination, .. } => Some(*destination),
            Instruction::ElementGet { destination, .. } => Some(*destination),
            Instruction::ElementAddr { destination, .. } => Some(*destination),
            Instruction::ElementSet { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::CallIndirect { destination, .. } => *destination,
            Instruction::ManagedAlloc { destination, .. } => Some(*destination),
            Instruction::ManagedAllocArray { destination, .. } => Some(*destination),
            Instruction::RawAlloc { destination, .. } => Some(*destination),
            Instruction::RawFree { .. } => None,
            Instruction::StackAlloc { destination, .. } => Some(*destination),
            Instruction::Intrinsic { destination, .. } => *destination,
        }
    }

    /// Get inline values used by this instruction (excludes externalized arguments).
    ///
    /// For Call, CallIndirect, and Intrinsic, the arguments are stored externally
    /// in NodeTree's argument buffer and must be fetched via `NodeTree::get_arguments()`.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        match self {
            Instruction::Const { .. } => smallvec![],
            Instruction::Binary { left, right, .. } => smallvec![*left, *right],
            Instruction::Unary { argument, .. } => smallvec![*argument],
            Instruction::Cast { argument, .. } => smallvec![*argument],
            Instruction::LocalGet { .. } => smallvec![],
            Instruction::LocalSet { value, .. } => smallvec![*value],
            Instruction::GlobalAddr { .. } => smallvec![],
            Instruction::GlobalConst { .. } => smallvec![],
            Instruction::Load { pointer, .. } => smallvec![*pointer],
            Instruction::Store { pointer, value, .. } => smallvec![*pointer, *value],
            Instruction::Drop { value } => smallvec![*value],
            Instruction::FieldGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldAddr { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::ElementGet { array, index, .. } => smallvec![*array, *index],
            Instruction::ElementAddr { array, index, .. } => smallvec![*array, *index],
            Instruction::ElementSet {
                array,
                index,
                value,
                ..
            } => smallvec![*array, *index, *value],
            // Arguments stored externally - return empty
            Instruction::Call { .. } => smallvec![],
            Instruction::CallIndirect { callee, .. } => smallvec![*callee],
            Instruction::ManagedAlloc { .. } => smallvec![],
            Instruction::ManagedAllocArray { length, .. } => smallvec![*length],
            Instruction::RawAlloc { .. } => smallvec![],
            Instruction::RawFree { pointer } => smallvec![*pointer],
            Instruction::StackAlloc { .. } => smallvec![],
            // Arguments stored externally - return empty
            Instruction::Intrinsic { .. } => smallvec![],
        }
    }

    /// Get the argument slice for instructions that have externalized arguments.
    ///
    /// Returns `Some(ArgumentSlice)` for Call, CallIndirect, and Intrinsic.
    /// Returns `None` for all other instructions.
    pub fn argument_slice(&self) -> Option<ArgumentSlice> {
        match self {
            Instruction::Call { arguments, .. } => Some(*arguments),
            Instruction::CallIndirect { arguments, .. } => Some(*arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }
}

/// Kind of type cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastOperator {
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

impl CastOperator {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            CastOperator::Bitcast => "bitcast",
            CastOperator::Truncate => "trunc",
            CastOperator::ZeroExtend => "uextend",
            CastOperator::SignExtend => "sextend",
            CastOperator::FloatToSignedInt => "fcvt_to_sint",
            CastOperator::FloatToUnsignedInt => "fcvt_to_uint",
            CastOperator::SignedIntToFloat => "scvt_to_float",
            CastOperator::UnsignedIntToFloat => "ucvt_to_float",
            CastOperator::FloatTruncate => "fnarrow",
            CastOperator::FloatExtend => "fwiden",
            CastOperator::PointerToInt => "ptr_to_int",
            CastOperator::IntToPointer => "int_to_ptr",
        }
    }
}

impl fmt::Display for CastOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for CastOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bitcast" => Ok(CastOperator::Bitcast),
            "trunc" => Ok(CastOperator::Truncate),
            "uextend" => Ok(CastOperator::ZeroExtend),
            "sextend" => Ok(CastOperator::SignExtend),
            "fcvt_to_sint" => Ok(CastOperator::FloatToSignedInt),
            "fcvt_to_uint" => Ok(CastOperator::FloatToUnsignedInt),
            "scvt_to_float" => Ok(CastOperator::SignedIntToFloat),
            "ucvt_to_float" => Ok(CastOperator::UnsignedIntToFloat),
            "fnarrow" => Ok(CastOperator::FloatTruncate),
            "fwiden" => Ok(CastOperator::FloatExtend),
            "ptr_to_int" => Ok(CastOperator::PointerToInt),
            "int_to_ptr" => Ok(CastOperator::IntToPointer),
            _ => Err(()),
        }
    }
}
