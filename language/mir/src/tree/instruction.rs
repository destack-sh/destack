use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    AtomicScope, BinaryOperator, CallEffects, Constant, Function, Global, Intrinsic, Local,
    LocalNodeId, MemoryOrdering, MemoryScope, MemorySemantics, Node, NodeType, TensorConvertMode,
    TensorConvolutionDimensionNumbers, TensorConvolutionWindow, TensorDotDimensionNumbers,
    TensorGatherDimensionNumbers, TensorReduceOperator, TensorScatterDimensionNumbers,
    TensorScatterMode, Type, UnaryOperator, Value, VectorConvertMode, VectorReduceOperator,
};

/// Compact representation of an argument slice stored in an external buffer.
///
/// Used by aggregate, call, intrinsic, and tensor instructions to reference value lists.
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

/// Dispatch kind for a call instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallDispatchKind {
    /// Direct function call.
    Direct,
    /// Virtual call through a vtable slot.
    Virtual {
        /// The vtable slot id for the method.
        slot_id: u32,
    },
    /// Interface call through an itab slot.
    Interface {
        /// The itab slot id for the method.
        slot_id: u32,
    },
    /// Indirect call through a function pointer.
    Indirect,
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

    // local variables (local.get, local.set, local.addr)
    /// Load from a local variable (stack slot).
    LocalGet {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The local variable to load from.
        local: LocalNodeId<Local>,
    },
    /// Get the address of a local variable (stack slot).
    LocalAddr {
        /// The SSA value to define with the local address.
        destination: Value,
        /// The local variable to take the address of.
        local: LocalNodeId<Local>,
        /// The result type of the address.
        result_type: LocalNodeId<Type>,
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
        /// The result type of the address.
        result_type: LocalNodeId<Type>,
    },
    /// Load the value of an immutable global constant.
    /// Returns the constant value directly.
    GlobalConst {
        /// The SSA value to define with the constant value.
        destination: Value,
        /// The global constant to load.
        global: LocalNodeId<Global>,
    },
    /// Get a function pointer for a function (function.addr).
    FunctionAddr {
        /// The SSA value to define with the function pointer.
        destination: Value,
        /// The function to take the address of.
        function: LocalNodeId<Function>,
    },
    /// Load the closure environment for the current function (function.env).
    FunctionEnv {
        /// The SSA value to define with the closure environment pointer.
        destination: Value,
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
        /// The loaded value type.
        result_type: LocalNodeId<Type>,
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
        /// The result type of the address.
        result_type: LocalNodeId<Type>,
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
        /// The result type of the address.
        result_type: LocalNodeId<Type>,
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

    // vector operations (vector.splat, vector.extract, vector.insert, vector.shuffle, vector.reduce)
    /// Broadcast a scalar to all vector lanes.
    VectorSplat {
        /// The SSA value to define with the vector result.
        destination: Value,
        /// The scalar value to broadcast.
        value: Value,
    },
    /// Extract a lane from a vector.
    VectorExtract {
        /// The SSA value to define with the extracted lane.
        destination: Value,
        /// The vector value to extract from.
        vector: Value,
        /// The lane index to extract.
        index: Value,
    },
    /// Insert a lane into a vector.
    VectorInsert {
        /// The SSA value to define with the updated vector.
        destination: Value,
        /// The original vector value.
        vector: Value,
        /// The lane index to update.
        index: Value,
        /// The lane value to insert.
        value: Value,
    },
    /// Shuffle vector lanes using a constant mask.
    VectorShuffle {
        /// The SSA value to define with the shuffled result.
        destination: Value,
        /// The left vector operand.
        left: Value,
        /// The right vector operand.
        right: Value,
        /// The shuffle mask indices.
        mask: Vec<u32>,
    },
    /// Reduce a vector to a scalar.
    VectorReduce {
        /// The SSA value to define with the reduced result.
        destination: Value,
        /// The reduction operator to apply.
        operator: VectorReduceOperator,
        /// The vector value to reduce.
        vector: Value,
    },
    /// Compare two vectors elementwise.
    ///
    /// The result is a vector of boolean lanes.
    VectorCompare {
        /// The SSA value to define with the comparison result.
        destination: Value,
        /// The comparison operator to apply.
        operator: BinaryOperator,
        /// The left vector operand.
        left: Value,
        /// The right vector operand.
        right: Value,
    },
    /// Convert vector element types with an explicit mode.
    ///
    /// The result must have the same lane count as the input.
    VectorConvert {
        /// The SSA value to define with the converted vector.
        destination: Value,
        /// The conversion mode to apply.
        mode: VectorConvertMode,
        /// The vector value to convert.
        vector: Value,
    },

    // tensor operations (tensor.*)
    /// Load a tensor element from a tensor reference.
    TensorLoad {
        /// The SSA value to define with the loaded element.
        destination: Value,
        /// The tensor reference to load from.
        view: Value,
        /// The index values (stored in NodeTree's argument buffer).
        indices: ArgumentSlice,
    },
    /// Store a tensor element into a tensor reference.
    TensorStore {
        /// The tensor reference to store into.
        view: Value,
        /// The index values (stored in NodeTree's argument buffer).
        indices: ArgumentSlice,
        /// The value to store.
        value: Value,
    },
    /// Fill a tensor reference with a scalar value.
    TensorFill {
        /// The tensor reference to fill.
        view: Value,
        /// The scalar value to write.
        value: Value,
    },
    /// Copy elements from a source tensor reference into a destination tensor reference.
    TensorCopy {
        /// The destination tensor reference.
        target: Value,
        /// The source tensor reference.
        source: Value,
    },
    /// Reshape a tensor value into a new shape.
    TensorReshape {
        /// The SSA value to define with the reshaped tensor.
        destination: Value,
        /// The tensor value to reshape.
        tensor: Value,
        /// The shape values (stored in NodeTree's argument buffer).
        shape: ArgumentSlice,
    },
    /// Broadcast a tensor into a larger shape.
    TensorBroadcast {
        /// The SSA value to define with the broadcasted tensor.
        destination: Value,
        /// The tensor value to broadcast.
        tensor: Value,
        /// The operand dimensions mapped into the result.
        dimensions: Vec<u32>,
    },
    /// Permute tensor dimensions.
    TensorTranspose {
        /// The SSA value to define with the transposed tensor.
        destination: Value,
        /// The tensor value to transpose.
        tensor: Value,
        /// The permutation of dimensions.
        permutation: Vec<u32>,
    },
    /// Refine a tensor type without changing its contents.
    TensorCast {
        /// The SSA value to define with the cast tensor.
        destination: Value,
        /// The tensor value to cast.
        tensor: Value,
    },
    /// Create a view into a tensor reference.
    TensorView {
        /// The SSA value to define with the view result.
        destination: Value,
        /// The tensor reference to view.
        view: Value,
        /// The view arguments (offsets, sizes, strides) stored in NodeTree's argument buffer.
        arguments: ArgumentSlice,
        /// The number of offset values.
        offsets_count: u16,
        /// The number of size values.
        sizes_count: u16,
        /// The number of stride values.
        strides_count: u16,
    },
    /// Slice a tensor by offsets, sizes, and strides.
    TensorSlice {
        /// The SSA value to define with the sliced tensor.
        destination: Value,
        /// The tensor value to slice.
        tensor: Value,
        /// The slice arguments (offsets, sizes, strides) stored in NodeTree's argument buffer.
        arguments: ArgumentSlice,
        /// The number of offset values.
        offsets_count: u16,
        /// The number of size values.
        sizes_count: u16,
        /// The number of stride values.
        strides_count: u16,
    },
    /// Pad a tensor with low, high, and interior padding.
    TensorPad {
        /// The SSA value to define with the padded tensor.
        destination: Value,
        /// The tensor value to pad.
        tensor: Value,
        /// The padding arguments (low, high, interior) stored in NodeTree's argument buffer.
        arguments: ArgumentSlice,
        /// The number of low padding values.
        low_count: u16,
        /// The number of high padding values.
        high_count: u16,
        /// The number of interior padding values.
        interior_count: u16,
        /// The scalar padding value.
        value: Value,
    },
    /// Concatenate tensors along a dimension.
    TensorConcat {
        /// The SSA value to define with the concatenated tensor.
        destination: Value,
        /// The tensor operands stored in NodeTree's argument buffer.
        tensors: ArgumentSlice,
        /// The concatenation axis.
        axis: u32,
    },
    /// Compare two tensors elementwise.
    ///
    /// The result is a tensor with boolean element type and matching shape.
    TensorCompare {
        /// The SSA value to define with the comparison result.
        destination: Value,
        /// The comparison operator to apply.
        operator: BinaryOperator,
        /// The left tensor operand.
        left: Value,
        /// The right tensor operand.
        right: Value,
    },
    /// Reduce a tensor along axes with a fixed operator.
    TensorReduce {
        /// The SSA value to define with the reduced tensor.
        destination: Value,
        /// The reduction operator to apply.
        operator: TensorReduceOperator,
        /// The tensor value to reduce.
        tensor: Value,
        /// The initial value for the reduction.
        initial: Value,
        /// The axes to reduce.
        axes: Vec<u32>,
    },
    /// Dot product of two tensors.
    TensorDot {
        /// The SSA value to define with the dot result.
        destination: Value,
        /// The left operand.
        left: Value,
        /// The right operand.
        right: Value,
        /// The dot dimension numbers.
        dimensions: TensorDotDimensionNumbers,
    },
    /// Convolution between an input tensor and a kernel tensor.
    TensorConvolution {
        /// The SSA value to define with the convolution result.
        destination: Value,
        /// The input tensor.
        input: Value,
        /// The kernel tensor.
        kernel: Value,
        /// The convolution dimension numbers.
        dimensions: TensorConvolutionDimensionNumbers,
        /// The convolution window parameters.
        window: TensorConvolutionWindow,
        /// The number of feature groups.
        feature_group_count: u32,
        /// The number of batch groups.
        batch_group_count: u32,
    },
    /// Gather slices from a tensor based on indices.
    TensorGather {
        /// The SSA value to define with the gathered tensor.
        destination: Value,
        /// The operand tensor.
        operand: Value,
        /// The indices tensor.
        indices: Value,
        /// The gather dimension numbers.
        dimensions: TensorGatherDimensionNumbers,
        /// The slice sizes for each operand dimension.
        slice_sizes: Vec<u32>,
    },
    /// Scatter updates into a tensor based on indices.
    TensorScatter {
        /// The SSA value to define with the scatter result.
        destination: Value,
        /// The operand tensor.
        operand: Value,
        /// The indices tensor.
        indices: Value,
        /// The updates tensor.
        updates: Value,
        /// The scatter dimension numbers.
        dimensions: TensorScatterDimensionNumbers,
        /// The scatter update mode.
        mode: TensorScatterMode,
    },
    /// Convert a tensor element type.
    ///
    /// The result must have the same shape as the input.
    TensorConvert {
        /// The SSA value to define with the converted tensor.
        destination: Value,
        /// The conversion mode to apply.
        mode: TensorConvertMode,
        /// The tensor value to convert.
        tensor: Value,
    },

    // function calls (call, call.virtual, call.interface, call.indirect)
    /// Call a function directly.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function to call.
        function: LocalNodeId<Function>,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
        /// The signature type for the callee.
        signature: LocalNodeId<Type>,
        /// Optional callsite effects and attributes.
        effects: Option<CallEffects>,
    },
    /// Call a virtual method through a vtable slot.
    CallVirtual {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The receiver value for dispatch.
        receiver: Value,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
        /// The declaring type for this virtual call.
        declaring_type: LocalNodeId<Type>,
        /// The vtable slot id for the method.
        slot_id: u32,
        /// The declared target function, when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The signature type for the callee.
        signature: LocalNodeId<Type>,
        /// Optional callsite effects and attributes.
        effects: Option<CallEffects>,
    },
    /// Call an interface method through an itab slot.
    CallInterface {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The receiver value for dispatch.
        receiver: Value,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
        /// The declaring interface type for this call.
        declaring_type: LocalNodeId<Type>,
        /// The itab slot id for the method.
        slot_id: u32,
        /// The declared target function, when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The signature type for the callee.
        signature: LocalNodeId<Type>,
        /// Optional callsite effects and attributes.
        effects: Option<CallEffects>,
    },
    /// Call through a function pointer (call.indirect).
    CallIndirect {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function pointer to call.
        callee: Value,
        /// Optional closure environment to pass to the callee.
        env: Option<Value>,
        /// The arguments to pass (stored in NodeTree's argument buffer).
        arguments: ArgumentSlice,
        /// The signature type for the callee.
        signature: LocalNodeId<Type>,
        /// Optional callsite effects and attributes.
        effects: Option<CallEffects>,
    },

    // allocation (managed - runtime tracks memory: managed.alloc, managed.alloc_array)
    /// Allocate a managed (runtime-tracked) struct (managed.alloc).
    /// Returns a `ref<managed T>`.
    ManagedAlloc {
        /// The SSA value to define with the allocated reference.
        destination: Value,
        /// The type of the struct to allocate.
        layout: LocalNodeId<Type>,
        /// The result type of the allocation.
        result_type: LocalNodeId<Type>,
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
        /// The result type of the allocation.
        result_type: LocalNodeId<Type>,
    },

    // allocation (raw - manual memory management: raw.alloc, raw.free, raw.drop)
    /// Allocate raw memory on the heap (raw.alloc).
    /// Returns a `ref<raw T>`. Caller must free with `raw.free` or `raw.drop`.
    RawAlloc {
        /// The SSA value to define with the allocated pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
        /// The result type of the allocation.
        result_type: LocalNodeId<Type>,
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

    // allocation (stack, automatic, scoped to function: stack.alloc, stack.drop)
    /// Allocate on the stack (lives until function returns) (stack.alloc).
    /// Returns a `ref<raw T>`. Freed automatically when frame exits.
    StackAlloc {
        /// The SSA value to define with the stack pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: LocalNodeId<Type>,
        /// The result type of the allocation.
        result_type: LocalNodeId<Type>,
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
        /// Execution scope for synchronization.
        scope: Option<AtomicScope>,
        /// Memory scope for synchronization.
        memory_scope: Option<MemoryScope>,
        /// Memory semantics for atomic operations and barriers.
        semantics: Option<MemorySemantics>,
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
            Instruction::LocalAddr { destination, .. } => Some(*destination),
            Instruction::LocalSet { .. } => None,
            Instruction::GlobalAddr { destination, .. } => Some(*destination),
            Instruction::GlobalConst { destination, .. } => Some(*destination),
            Instruction::FunctionAddr { destination, .. } => Some(*destination),
            Instruction::FunctionEnv { destination, .. } => Some(*destination),
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
            Instruction::VectorSplat { destination, .. } => Some(*destination),
            Instruction::VectorExtract { destination, .. } => Some(*destination),
            Instruction::VectorInsert { destination, .. } => Some(*destination),
            Instruction::VectorShuffle { destination, .. } => Some(*destination),
            Instruction::VectorReduce { destination, .. } => Some(*destination),
            Instruction::VectorCompare { destination, .. } => Some(*destination),
            Instruction::VectorConvert { destination, .. } => Some(*destination),
            Instruction::TensorLoad { destination, .. } => Some(*destination),
            Instruction::TensorStore { .. } => None,
            Instruction::TensorFill { .. } => None,
            Instruction::TensorCopy { .. } => None,
            Instruction::TensorReshape { destination, .. } => Some(*destination),
            Instruction::TensorBroadcast { destination, .. } => Some(*destination),
            Instruction::TensorTranspose { destination, .. } => Some(*destination),
            Instruction::TensorCast { destination, .. } => Some(*destination),
            Instruction::TensorView { destination, .. } => Some(*destination),
            Instruction::TensorSlice { destination, .. } => Some(*destination),
            Instruction::TensorPad { destination, .. } => Some(*destination),
            Instruction::TensorConcat { destination, .. } => Some(*destination),
            Instruction::TensorCompare { destination, .. } => Some(*destination),
            Instruction::TensorReduce { destination, .. } => Some(*destination),
            Instruction::TensorDot { destination, .. } => Some(*destination),
            Instruction::TensorConvolution { destination, .. } => Some(*destination),
            Instruction::TensorGather { destination, .. } => Some(*destination),
            Instruction::TensorScatter { destination, .. } => Some(*destination),
            Instruction::TensorConvert { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::CallVirtual { destination, .. } => *destination,
            Instruction::CallInterface { destination, .. } => *destination,
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
    /// For Call, CallVirtual, CallInterface, CallIndirect, and Intrinsic, the arguments are stored externally
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
            Instruction::LocalAddr { .. } => smallvec![],
            Instruction::LocalSet { value, .. } => smallvec![*value],
            Instruction::GlobalAddr { .. } => smallvec![],
            Instruction::GlobalConst { .. } => smallvec![],
            Instruction::FunctionAddr { .. } => smallvec![],
            Instruction::FunctionEnv { .. } => smallvec![],
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
            Instruction::VectorSplat { value, .. } => smallvec![*value],
            Instruction::VectorExtract { vector, index, .. } => smallvec![*vector, *index],
            Instruction::VectorInsert {
                vector,
                index,
                value,
                ..
            } => smallvec![*vector, *index, *value],
            Instruction::VectorShuffle { left, right, .. } => smallvec![*left, *right],
            Instruction::VectorReduce { vector, .. } => smallvec![*vector],
            Instruction::VectorCompare { left, right, .. } => smallvec![*left, *right],
            Instruction::VectorConvert { vector, .. } => smallvec![*vector],
            Instruction::TensorLoad { view, .. } => smallvec![*view],
            Instruction::TensorStore { view, value, .. } => smallvec![*view, *value],
            Instruction::TensorFill { view, value } => smallvec![*view, *value],
            Instruction::TensorCopy { target, source } => smallvec![*target, *source],
            Instruction::TensorReshape { tensor, .. } => smallvec![*tensor],
            Instruction::TensorBroadcast { tensor, .. } => smallvec![*tensor],
            Instruction::TensorTranspose { tensor, .. } => smallvec![*tensor],
            Instruction::TensorCast { tensor, .. } => smallvec![*tensor],
            Instruction::TensorView { view, .. } => smallvec![*view],
            Instruction::TensorSlice { tensor, .. } => smallvec![*tensor],
            Instruction::TensorPad { tensor, value, .. } => smallvec![*tensor, *value],
            Instruction::TensorConcat { .. } => smallvec![],
            Instruction::TensorCompare { left, right, .. } => smallvec![*left, *right],
            Instruction::TensorReduce {
                tensor, initial, ..
            } => smallvec![*tensor, *initial],
            Instruction::TensorDot { left, right, .. } => smallvec![*left, *right],
            Instruction::TensorConvolution { input, kernel, .. } => {
                smallvec![*input, *kernel]
            }
            Instruction::TensorGather {
                operand, indices, ..
            } => {
                smallvec![*operand, *indices]
            }
            Instruction::TensorScatter {
                operand,
                indices,
                updates,
                ..
            } => smallvec![*operand, *indices, *updates],
            Instruction::TensorConvert { tensor, .. } => smallvec![*tensor],
            Instruction::Call { .. } => smallvec![],
            Instruction::CallVirtual { receiver, .. } => smallvec![*receiver],
            Instruction::CallInterface { receiver, .. } => smallvec![*receiver],
            Instruction::CallIndirect { callee, env, .. } => {
                let mut values = SmallVec::new();
                values.push(*callee);
                if let Some(env) = env {
                    values.push(*env);
                }
                values
            }
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
    /// Returns `Some(ArgumentSlice)` for Struct, Tuple, Array, Call, CallVirtual, CallInterface,
    /// CallIndirect, Intrinsic, and tensor instructions that externalize value lists.
    /// Returns `None` for all other instructions.
    pub fn argument_slice(&self) -> Option<ArgumentSlice> {
        match self {
            Instruction::Struct { fields, .. } => Some(*fields),
            Instruction::Tuple { elements, .. } => Some(*elements),
            Instruction::Array { elements, .. } => Some(*elements),
            Instruction::TensorLoad { indices, .. } => Some(*indices),
            Instruction::TensorStore { indices, .. } => Some(*indices),
            Instruction::TensorReshape { shape, .. } => Some(*shape),
            Instruction::TensorView { arguments, .. } => Some(*arguments),
            Instruction::TensorSlice { arguments, .. } => Some(*arguments),
            Instruction::TensorPad { arguments, .. } => Some(*arguments),
            Instruction::TensorConcat { tensors, .. } => Some(*tensors),
            Instruction::Call { arguments, .. } => Some(*arguments),
            Instruction::CallVirtual { arguments, .. } => Some(*arguments),
            Instruction::CallInterface { arguments, .. } => Some(*arguments),
            Instruction::CallIndirect { arguments, .. } => Some(*arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }

    /// Return the dispatch kind for call instructions.
    pub fn call_dispatch_kind(&self) -> Option<CallDispatchKind> {
        match self {
            Instruction::Call { .. } => Some(CallDispatchKind::Direct),
            Instruction::CallVirtual { slot_id, .. } => {
                Some(CallDispatchKind::Virtual { slot_id: *slot_id })
            }
            Instruction::CallInterface { slot_id, .. } => {
                Some(CallDispatchKind::Interface { slot_id: *slot_id })
            }
            Instruction::CallIndirect { .. } => Some(CallDispatchKind::Indirect),
            _ => None,
        }
    }

    /// Return the signature type for call instructions.
    pub fn call_signature(&self) -> Option<LocalNodeId<Type>> {
        match self {
            Instruction::Call { signature, .. }
            | Instruction::CallVirtual { signature, .. }
            | Instruction::CallInterface { signature, .. }
            | Instruction::CallIndirect { signature, .. } => Some(*signature),
            _ => None,
        }
    }

    /// Return the declared target for call instructions, when known.
    pub fn call_declared_target(&self) -> Option<LocalNodeId<Function>> {
        match self {
            Instruction::Call { function, .. } => Some(*function),
            Instruction::CallVirtual {
                declared_target, ..
            }
            | Instruction::CallInterface {
                declared_target, ..
            } => *declared_target,
            _ => None,
        }
    }

    /// Return callsite effects for call instructions, when present.
    pub fn call_effects(&self) -> Option<&CallEffects> {
        match self {
            Instruction::Call { effects, .. }
            | Instruction::CallVirtual { effects, .. }
            | Instruction::CallInterface { effects, .. }
            | Instruction::CallIndirect { effects, .. } => effects.as_ref(),
            _ => None,
        }
    }

    /// Return mutable callsite effects for call instructions, when present.
    pub fn call_effects_mut(&mut self) -> Option<&mut CallEffects> {
        match self {
            Instruction::Call { effects, .. }
            | Instruction::CallVirtual { effects, .. }
            | Instruction::CallInterface { effects, .. }
            | Instruction::CallIndirect { effects, .. } => effects.as_mut(),
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
