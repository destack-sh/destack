//! MIR instructions.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Constant, Function, Global, Intrinsic, Local, LocalNodeId, MemoryOrdering,
    Node, NodeType, Type, UnaryOperator, Value,
};

/// Compact representation of an argument slice stored in an external buffer.
///
/// Used by Call, CallIndirect, and Intrinsic instructions to reference arguments.
/// This saves 16 bytes per instruction compared to using `Vec<Value>` inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    // conditional selection
    /// Select a value based on a boolean condition.
    ///
    /// Returns `then_value` if `condition` is true, `else_value` otherwise.
    /// Both values must have the same type. Unlike a branch, both values are
    /// computed before the selection (no short-circuit evaluation).
    Select {
        /// The SSA value to define with the selected result.
        destination: Value,
        /// The boolean condition (must be bool type).
        condition: Value,
        /// The value returned if condition is true.
        then_value: Value,
        /// The value returned if condition is false.
        else_value: Value,
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
    ///
    /// Optional memory access metadata is stored in `NodeTree::memory_table`.
    Load {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The pointer to load from.
        pointer: Value,
    },
    /// Store to a pointer (write through pointer).
    ///
    /// Optional memory access metadata is stored in `NodeTree::memory_table`.
    Store {
        /// The pointer to store to.
        pointer: Value,
        /// The value to store.
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
    /// Construct a struct from field values.
    ///
    /// Fields must be provided in layout order.
    Struct {
        /// The SSA value to define with the constructed struct.
        destination: Value,
        /// The struct type to construct.
        ty: LocalNodeId<Type>,
        /// The field values (stored in NodeTree's argument buffer).
        fields: ArgumentSlice,
    },
    /// Construct a tuple from element values.
    ///
    /// Elements must be provided in order.
    Tuple {
        /// The SSA value to define with the constructed tuple.
        destination: Value,
        /// The tuple type to construct.
        ty: LocalNodeId<Type>,
        /// The element values (stored in NodeTree's argument buffer).
        elements: ArgumentSlice,
    },
    /// Construct an array from element values.
    ///
    /// Elements must be provided in index order.
    Array {
        /// The SSA value to define with the constructed array.
        destination: Value,
        /// The array type to construct.
        ty: LocalNodeId<Type>,
        /// The element values (stored in NodeTree's argument buffer).
        elements: ArgumentSlice,
    },

    // function calls (call, call.indirect)
    /// Call a function directly.
    ///
    /// Callsite metadata, including memory effects, is stored in `NodeTree::call_table`.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function to call.
        function: LocalNodeId<Function>,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
    },
    /// Call through a function pointer (call.indirect).
    ///
    /// Callsite metadata, including memory effects, is stored in `NodeTree::call_table`.
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

    // allocation (raw - manual memory management: raw.alloc, raw.free, raw.drop)
    /// Allocate raw memory on the heap (raw.alloc).
    /// Returns a `ref<raw T>`. Caller must free with `raw.free` or `raw.drop`.
    RawAlloc {
        /// The SSA value to define with the allocated pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
    },
    /// Free raw heap memory previously allocated with `raw.alloc` (raw.free).
    /// User-inserted for manual memory management (FFI, etc).
    RawFree {
        /// The pointer to free.
        pointer: Value,
    },
    /// Drop an owned heap value (raw.drop).
    /// Compiler-inserted to deallocate heap memory at ownership end.
    /// Dispose and field drops are explicit calls preceding this.
    RawDrop {
        /// The value to drop.
        value: Value,
    },

    // allocation (stack - automatic, scoped to function: stack.alloc, stack.drop)
    /// Allocate on the stack (lives until function returns) (stack.alloc).
    /// Returns a `ref<raw T>`. Freed automatically when frame exits.
    StackAlloc {
        /// The SSA value to define with the stack pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
    },
    /// Mark a stack value's lifetime as ended (stack.drop).
    /// Compiler-inserted for NLL. No deallocation (frame handles it).
    StackDrop {
        /// The value to drop.
        value: Value,
    },

    // assumptions and hints
    /// Assume a condition is true (UB if false).
    Assume {
        /// The condition to assume.
        condition: Value,
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
            Instruction::Select { destination, .. } => Some(*destination),
            Instruction::LocalGet { destination, .. } => Some(*destination),
            Instruction::LocalSet { .. } => None,
            Instruction::GlobalAddr { destination, .. } => Some(*destination),
            Instruction::GlobalConst { destination, .. } => Some(*destination),
            Instruction::Load { destination, .. } => Some(*destination),
            Instruction::Store { .. } => None,
            Instruction::FieldGet { destination, .. } => Some(*destination),
            Instruction::FieldAddr { destination, .. } => Some(*destination),
            Instruction::FieldSet { destination, .. } => Some(*destination),
            Instruction::ElementGet { destination, .. } => Some(*destination),
            Instruction::ElementAddr { destination, .. } => Some(*destination),
            Instruction::ElementSet { destination, .. } => Some(*destination),
            Instruction::Struct { destination, .. } => Some(*destination),
            Instruction::Tuple { destination, .. } => Some(*destination),
            Instruction::Array { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::CallIndirect { destination, .. } => *destination,
            Instruction::ManagedAlloc { destination, .. } => Some(*destination),
            Instruction::ManagedAllocArray { destination, .. } => Some(*destination),
            Instruction::RawAlloc { destination, .. } => Some(*destination),
            Instruction::RawFree { .. } => None,
            Instruction::RawDrop { .. } => None,
            Instruction::StackAlloc { destination, .. } => Some(*destination),
            Instruction::StackDrop { .. } => None,
            Instruction::Assume { .. } => None,
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
            Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => smallvec![*condition, *then_value, *else_value],
            Instruction::LocalGet { .. } => smallvec![],
            Instruction::LocalSet { value, .. } => smallvec![*value],
            Instruction::GlobalAddr { .. } => smallvec![],
            Instruction::GlobalConst { .. } => smallvec![],
            Instruction::Load { pointer, .. } => smallvec![*pointer],
            Instruction::Store { pointer, value, .. } => smallvec![*pointer, *value],
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
            // arguments stored externally - return empty
            Instruction::Struct { .. } => smallvec![],
            Instruction::Tuple { .. } => smallvec![],
            Instruction::Array { .. } => smallvec![],
            Instruction::Call { .. } => smallvec![],
            Instruction::CallIndirect { callee, .. } => smallvec![*callee],
            Instruction::ManagedAlloc { .. } => smallvec![],
            Instruction::ManagedAllocArray { length, .. } => smallvec![*length],
            Instruction::RawAlloc { .. } => smallvec![],
            Instruction::RawFree { pointer } => smallvec![*pointer],
            Instruction::RawDrop { value } => smallvec![*value],
            Instruction::StackAlloc { .. } => smallvec![],
            Instruction::StackDrop { value } => smallvec![*value],
            Instruction::Assume { condition } => smallvec![*condition],
            // Arguments stored externally - return empty
            Instruction::Intrinsic { .. } => smallvec![],
        }
    }

    /// Get the argument slice for instructions that have externalized arguments.
    ///
    /// Returns `Some(ArgumentSlice)` for Struct, Tuple, Array, Call, CallIndirect, and Intrinsic.
    /// Returns `None` for all other instructions.
    pub fn argument_slice(&self) -> Option<ArgumentSlice> {
        match self {
            Instruction::Struct { fields, .. } => Some(*fields),
            Instruction::Tuple { elements, .. } => Some(*elements),
            Instruction::Array { elements, .. } => Some(*elements),
            Instruction::Call { arguments, .. } => Some(*arguments),
            Instruction::CallIndirect { arguments, .. } => Some(*arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }
}

/// Kind of type cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
