use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    AllocationSize, ArgumentAttribute, AtomicAccess, AtomicRmwOperator, BinaryOperator, Call,
    CallBehavior, CompareExchangeAccess, Constant, DispatchSlot, FenceAccess, FunctionReference,
    GlobalReference, Intrinsic, LocalReference, MemoryEffect, Node, NodeType, PointerAttribute,
    TensorConvertMode, TensorConvolutionDimensionNumbers, TensorConvolutionWindow,
    TensorDotDimensionNumbers, TensorGatherDimensionNumbers, TensorReduceOperator,
    TensorScatterDimensionNumbers, TensorScatterMode, TypeReference, UnaryOperator, ValueReference,
    VectorConvertMode, VectorReduceOperator,
};

/// Compact representation of an argument slice stored in an external buffer.
///
/// Used by aggregate, call, intrinsic, and tensor instructions to reference value references.
/// (This saves 16 bytes per instruction compared to using `Vec<ValueReference>` inline.)
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
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Interface call through an itab slot.
    Interface {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Indirect call through a function pointer.
    Indirect,
}

/// Instructions produce SSA values and perform "operations".
/// Each instruction produces at most one value via the `destination` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    /// Recovered invalid instruction syntax.
    Error,

    // constants
    /// Load a constant value.
    Const {
        /// The SSA value to define.
        destination: ValueReference,
        /// The constant value to load.
        value: Constant,
    },

    // arithmetic
    /// Binary operation (e.g., add, subtract, compare).
    Binary {
        /// The SSA value to define with the result.
        destination: ValueReference,
        /// The binary operator to apply.
        operator: BinaryOperator,
        /// The left-hand operand.
        left: ValueReference,
        /// The right-hand operand.
        right: ValueReference,
    },
    /// Unary operation (e.g., negate, not).
    Unary {
        /// The SSA value to define with the result.
        destination: ValueReference,
        /// The unary operator to apply.
        operator: UnaryOperator,
        /// The operand.
        argument: ValueReference,
    },

    // type conversions
    /// Cast between types (bitcast, truncate, extend, etc.).
    Cast {
        /// The SSA value to define with the converted result.
        destination: ValueReference,
        /// The cast operator to perform.
        operator: CastOperator,
        /// The value to cast.
        argument: ValueReference,
        /// The target type to cast to.
        to_type: TypeReference,
    },

    // conditional selection
    /// Select a value based on a boolean condition.
    ///
    /// Returns `then_value` if `condition` is true, `else_value` otherwise.
    /// Both values must have the same type. Unlike a branch, both values are
    /// computed before the selection (no short-circuit evaluation).
    Select {
        /// The SSA value to define with the selected result.
        destination: ValueReference,
        /// The boolean condition (must be bool type).
        condition: ValueReference,
        /// The value returned if condition is true.
        then_value: ValueReference,
        /// The value returned if condition is false.
        else_value: ValueReference,
    },

    // local variables (local.get, local.set, local.address)
    /// Load from a local variable (stack slot).
    LocalGet {
        /// The SSA value to define with the loaded value.
        destination: ValueReference,
        /// The local variable to load from.
        local: LocalReference,
    },
    /// Get the address of a local variable (stack slot).
    LocalAddr {
        /// The SSA value to define with the local address.
        destination: ValueReference,
        /// The local variable to take the address of.
        local: LocalReference,
        /// The result type of the address.
        result_type: TypeReference,
    },
    /// Store to a local variable (stack slot).
    LocalSet {
        /// The local variable to store to.
        local: LocalReference,
        /// The value to store.
        value: ValueReference,
    },

    // global variables (global.address)
    /// Get the address of a mutable global variable.
    /// Returns a raw pointer that can be used with Load/Store.
    GlobalAddr {
        /// The SSA value to define with the pointer.
        destination: ValueReference,
        /// The global variable to get the address of.
        global: GlobalReference,
        /// The result type of the address.
        result_type: TypeReference,
    },
    /// Get a function pointer for a function (function.address).
    FunctionAddr {
        /// The SSA value to define with the function pointer.
        destination: ValueReference,
        /// The function to take the address of.
        function: FunctionReference,
    },
    /// Bind one environment to a function and produce a callable value (callable.bind).
    CallableBind {
        /// The SSA value to define with the callable value.
        destination: ValueReference,
        /// The function to pair with the environment.
        function: FunctionReference,
        /// The environment value to capture in the callable.
        environment: ValueReference,
    },
    /// Load the hidden environment for the current function (callable.environment).
    CallableEnvironment {
        /// The SSA value to define with the hidden environment pointer.
        destination: ValueReference,
    },

    // memory (pointers)
    /// Load from a pointer (dereference).
    ///
    /// Optional memory access metadata is stored in `Tree::memory_table`.
    Load {
        /// The SSA value to define with the loaded value.
        destination: ValueReference,
        /// The pointer to load from.
        pointer: ValueReference,
        /// The loaded value type.
        result_type: TypeReference,
    },
    /// Store to a pointer (write through pointer).
    ///
    /// Optional memory access metadata is stored in `Tree::memory_table`.
    Store {
        /// The pointer to store to.
        pointer: ValueReference,
        /// The value to store.
        value: ValueReference,
    },

    // aggregate operations (field.get, field.address, field.set, element.get, element.address, element.set)
    /// Extract a field from an aggregate value (field.get).
    FieldGet {
        /// The SSA value to define with the extracted field.
        destination: ValueReference,
        /// The aggregate value to extract from.
        aggregate: ValueReference,
        /// The zero-based field index.
        index: u32,
    },
    /// Get the address of a field from an addressable aggregate (field.address).
    FieldAddr {
        /// The SSA value to define with the field address.
        destination: ValueReference,
        /// The aggregate base to project from.
        aggregate: ValueReference,
        /// The zero-based field index.
        index: u32,
        /// The result type of the address.
        result_type: TypeReference,
    },
    /// Insert a value into a struct or tuple field (field.set).
    FieldSet {
        /// The SSA value to define with the new aggregate.
        destination: ValueReference,
        /// The original aggregate value.
        aggregate: ValueReference,
        /// The zero-based field index to update.
        index: u32,
        /// The value to insert at the field.
        value: ValueReference,
    },
    /// Extract an element from an array aggregate (element.get).
    ElementGet {
        /// The SSA value to define with the extracted element.
        destination: ValueReference,
        /// The array value to extract from.
        array: ValueReference,
        /// The zero-based element index.
        index: u32,
    },
    /// Get the address of an element from an addressable indexed value (element.address).
    ElementAddr {
        /// The SSA value to define with the element address.
        destination: ValueReference,
        /// The indexed base to project from.
        array: ValueReference,
        /// The index of the element (runtime value).
        index: ValueReference,
        /// The result type of the address.
        result_type: TypeReference,
    },
    /// Insert a value into an array element (element.set).
    ElementSet {
        /// The SSA value to define with the new array.
        destination: ValueReference,
        /// The original array value.
        array: ValueReference,
        /// The zero-based element index to update.
        index: u32,
        /// The value to insert at the index.
        value: ValueReference,
    },
    /// Construct a struct from field values.
    ///
    /// Fields must be provided in layout order.
    Struct {
        /// The SSA value to define with the constructed struct.
        destination: ValueReference,
        /// The struct type to construct.
        ty: TypeReference,
        /// The field values (stored in Tree's argument buffer).
        fields: ArgumentSlice,
    },
    /// Construct a tuple from element values.
    ///
    /// Elements must be provided in order.
    Tuple {
        /// The SSA value to define with the constructed tuple.
        destination: ValueReference,
        /// The tuple type to construct.
        ty: TypeReference,
        /// The element values (stored in Tree's argument buffer).
        elements: ArgumentSlice,
    },
    /// Construct an array from element values.
    ///
    /// Elements must be provided in index order.
    Array {
        /// The SSA value to define with the constructed array.
        destination: ValueReference,
        /// The array type to construct.
        ty: TypeReference,
        /// The element values (stored in Tree's argument buffer).
        elements: ArgumentSlice,
    },

    // vector operations (vector.splat, vector.extract, vector.insert, vector.shuffle, vector.reduce)
    /// Broadcast a scalar to all vector lanes.
    VectorSplat {
        /// The SSA value to define with the vector result.
        destination: ValueReference,
        /// The scalar value to broadcast.
        value: ValueReference,
    },
    /// Extract a lane from a vector.
    VectorExtract {
        /// The SSA value to define with the extracted lane.
        destination: ValueReference,
        /// The vector value to extract from.
        vector: ValueReference,
        /// The lane index to extract.
        index: ValueReference,
    },
    /// Insert a lane into a vector.
    VectorInsert {
        /// The SSA value to define with the updated vector.
        destination: ValueReference,
        /// The original vector value.
        vector: ValueReference,
        /// The lane index to update.
        index: ValueReference,
        /// The lane value to insert.
        value: ValueReference,
    },
    /// Shuffle vector lanes using a constant mask.
    VectorShuffle {
        /// The SSA value to define with the shuffled result.
        destination: ValueReference,
        /// The left vector operand.
        left: ValueReference,
        /// The right vector operand.
        right: ValueReference,
        /// The shuffle mask indices.
        mask: Vec<u32>,
    },
    /// Select vector lanes based on a boolean mask.
    VectorSelect {
        /// The SSA value to define with the selected result.
        destination: ValueReference,
        /// The boolean mask vector.
        mask: ValueReference,
        /// The value returned if the mask lane is true.
        then_value: ValueReference,
        /// The value returned if the mask lane is false.
        else_value: ValueReference,
    },
    /// Reduce a vector to a scalar.
    VectorReduce {
        /// The SSA value to define with the reduced result.
        destination: ValueReference,
        /// The reduction operator to apply.
        operator: VectorReduceOperator,
        /// The vector value to reduce.
        vector: ValueReference,
    },
    /// Compare two vectors elementwise.
    ///
    /// The result is a vector of boolean lanes.
    VectorCompare {
        /// The SSA value to define with the comparison result.
        destination: ValueReference,
        /// The comparison operator to apply.
        operator: BinaryOperator,
        /// The left vector operand.
        left: ValueReference,
        /// The right vector operand.
        right: ValueReference,
    },
    /// Convert vector element types with an explicit mode.
    ///
    /// The result must have the same lane count as the input.
    VectorConvert {
        /// The SSA value to define with the converted vector.
        destination: ValueReference,
        /// The conversion mode to apply.
        mode: VectorConvertMode,
        /// The vector value to convert.
        vector: ValueReference,
    },

    // tensor operations (tensor.*)
    /// Broadcast a scalar to all tensor elements.
    TensorSplat {
        /// The SSA value to define with the tensor result.
        destination: ValueReference,
        /// The scalar value to broadcast.
        value: ValueReference,
    },
    /// Load a tensor element from a tensor reference.
    TensorLoad {
        /// The SSA value to define with the loaded element.
        destination: ValueReference,
        /// The tensor reference to load from.
        view: ValueReference,
        /// The index values (stored in Tree's argument buffer).
        indices: ArgumentSlice,
    },
    /// Extract a tensor element from a tensor value.
    TensorExtract {
        /// The SSA value to define with the extracted element.
        destination: ValueReference,
        /// The tensor value to extract from.
        tensor: ValueReference,
        /// The index values (stored in Tree's argument buffer).
        indices: ArgumentSlice,
    },
    /// Store a tensor element into a tensor reference.
    TensorStore {
        /// The tensor reference to store into.
        view: ValueReference,
        /// The index values (stored in Tree's argument buffer).
        indices: ArgumentSlice,
        /// The value to store.
        value: ValueReference,
    },
    /// Fill a tensor reference with a scalar value.
    TensorFill {
        /// The tensor reference to fill.
        view: ValueReference,
        /// The scalar value to write.
        value: ValueReference,
    },
    /// Copy elements from a source tensor reference into a destination tensor reference.
    TensorCopy {
        /// The destination tensor reference.
        target: ValueReference,
        /// The source tensor reference.
        source: ValueReference,
    },
    /// Reshape a tensor value into a new shape.
    TensorReshape {
        /// The SSA value to define with the reshaped tensor.
        destination: ValueReference,
        /// The tensor value to reshape.
        tensor: ValueReference,
        /// The shape values (stored in Tree's argument buffer).
        shape: ArgumentSlice,
    },
    /// Broadcast a tensor into a larger shape.
    TensorBroadcast {
        /// The SSA value to define with the broadcasted tensor.
        destination: ValueReference,
        /// The tensor value to broadcast.
        tensor: ValueReference,
        /// The operand dimensions mapped into the result.
        dimensions: Vec<u32>,
    },
    /// Permute tensor dimensions.
    TensorTranspose {
        /// The SSA value to define with the transposed tensor.
        destination: ValueReference,
        /// The tensor value to transpose.
        tensor: ValueReference,
        /// The permutation of dimensions.
        permutation: Vec<u32>,
    },
    /// Refine a tensor type without changing its contents.
    TensorCast {
        /// The SSA value to define with the cast tensor.
        destination: ValueReference,
        /// The tensor value to cast.
        tensor: ValueReference,
    },
    /// Create a view into a tensor reference.
    TensorView {
        /// The SSA value to define with the view result.
        destination: ValueReference,
        /// The tensor reference to view.
        view: ValueReference,
        /// The view arguments (offsets, sizes, strides) stored in Tree's argument buffer.
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
        destination: ValueReference,
        /// The tensor value to slice.
        tensor: ValueReference,
        /// The slice arguments (offsets, sizes, strides) stored in Tree's argument buffer.
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
        destination: ValueReference,
        /// The tensor value to pad.
        tensor: ValueReference,
        /// The padding arguments (low, high, interior) stored in Tree's argument buffer.
        arguments: ArgumentSlice,
        /// The number of low padding values.
        low_count: u16,
        /// The number of high padding values.
        high_count: u16,
        /// The number of interior padding values.
        interior_count: u16,
        /// The scalar padding value.
        value: ValueReference,
    },
    /// Concatenate tensors along a dimension.
    TensorConcat {
        /// The SSA value to define with the concatenated tensor.
        destination: ValueReference,
        /// The tensor operands stored in Tree's argument buffer.
        tensors: ArgumentSlice,
        /// The concatenation axis.
        axis: u32,
    },
    /// Compare two tensors elementwise.
    ///
    /// The result is a tensor with boolean element type and matching shape.
    TensorCompare {
        /// The SSA value to define with the comparison result.
        destination: ValueReference,
        /// The comparison operator to apply.
        operator: BinaryOperator,
        /// The left tensor operand.
        left: ValueReference,
        /// The right tensor operand.
        right: ValueReference,
    },
    /// Select tensor elements based on a boolean mask.
    TensorSelect {
        /// The SSA value to define with the selected tensor.
        destination: ValueReference,
        /// The boolean mask tensor.
        mask: ValueReference,
        /// The tensor returned if the mask element is true.
        then_value: ValueReference,
        /// The tensor returned if the mask element is false.
        else_value: ValueReference,
    },
    /// Reduce a tensor along axes with a fixed operator.
    TensorReduce {
        /// The SSA value to define with the reduced tensor.
        destination: ValueReference,
        /// The reduction operator to apply.
        operator: TensorReduceOperator,
        /// The tensor value to reduce.
        tensor: ValueReference,
        /// The initial value for the reduction.
        initial: ValueReference,
        /// The axes to reduce.
        axes: Vec<u32>,
    },
    /// Dot product of two tensors.
    TensorDot {
        /// The SSA value to define with the dot result.
        destination: ValueReference,
        /// The left operand.
        left: ValueReference,
        /// The right operand.
        right: ValueReference,
        /// The dot dimension numbers.
        dimensions: TensorDotDimensionNumbers,
    },
    /// Convolution between an input tensor and a kernel tensor.
    TensorConvolution {
        /// The SSA value to define with the convolution result.
        destination: ValueReference,
        /// The input tensor.
        input: ValueReference,
        /// The kernel tensor.
        kernel: ValueReference,
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
        destination: ValueReference,
        /// The operand tensor.
        operand: ValueReference,
        /// The indices tensor.
        indices: ValueReference,
        /// The gather dimension numbers.
        dimensions: TensorGatherDimensionNumbers,
        /// The slice sizes for each operand dimension.
        slice_sizes: Vec<u32>,
    },
    /// Scatter updates into a tensor based on indices.
    TensorScatter {
        /// The SSA value to define with the scatter result.
        destination: ValueReference,
        /// The operand tensor.
        operand: ValueReference,
        /// The indices tensor.
        indices: ValueReference,
        /// The updates tensor.
        updates: ValueReference,
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
        destination: ValueReference,
        /// The conversion mode to apply.
        mode: TensorConvertMode,
        /// The tensor value to convert.
        tensor: ValueReference,
    },

    // function calls (call, call.virtual, call.interface, call.indirect)
    /// Call a function directly.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<ValueReference>,
        /// The function to call.
        function: FunctionReference,
        /// The shared call payload.
        call: Call<ArgumentSlice>,
    },
    /// Call a virtual method through a vtable slot.
    CallVirtual {
        /// The SSA value to define with the return value, if any.
        destination: Option<ValueReference>,
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring type for this virtual call.
        declaring_type: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<ArgumentSlice>,
    },
    /// Call an interface method through an itab slot.
    CallInterface {
        /// The SSA value to define with the return value, if any.
        destination: Option<ValueReference>,
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring interface type for this call.
        declaring_type: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<ArgumentSlice>,
    },
    /// Call through a function pointer (call.indirect).
    CallIndirect {
        /// The SSA value to define with the return value, if any.
        destination: Option<ValueReference>,
        /// The callable value to call.
        callee: ValueReference,
        /// The shared call payload.
        call: Call<ArgumentSlice>,
    },

    // heap allocation
    /// Allocate typed heap storage (`new`).
    ///
    /// The result type decides whether the returned reference is managed or owned.
    New {
        /// The SSA value to define with the allocated reference.
        destination: ValueReference,
        /// The type of the struct to allocate.
        layout: TypeReference,
        /// The result type of the allocation.
        result_type: TypeReference,
    },
    /// Allocate typed repeated heap storage (`new.slice`).
    ///
    /// The result type decides whether the returned slice is managed or owned.
    NewSlice {
        /// The SSA value to define with the allocated slice.
        destination: ValueReference,
        /// The element type of the repeated storage.
        element: TypeReference,
        /// The number of elements (runtime value).
        length: ValueReference,
        /// The result type of the allocation.
        result_type: TypeReference,
    },

    // raw allocation
    /// Allocate raw heap storage (`raw.alloc`).
    ///
    /// The caller must release the result with `raw.free`.
    RawAlloc {
        /// The SSA value to define with the allocated pointer.
        destination: ValueReference,
        /// The type of the value to allocate.
        layout: TypeReference,
        /// The result type of the allocation.
        result_type: TypeReference,
    },
    /// Release raw heap storage (`raw.free`).
    ///
    /// This is only valid for raw references produced by `raw.alloc`.
    RawFree {
        /// The pointer to free.
        pointer: ValueReference,
    },

    // stack allocation
    /// Allocate frame-scoped stack storage (`stack.alloc`).
    ///
    /// The storage is released when the frame exits.
    StackAlloc {
        /// The SSA value to define with the stack pointer.
        destination: ValueReference,
        /// The type of the value to allocate.
        layout: TypeReference,
        /// The result type of the allocation.
        result_type: TypeReference,
    },

    // ownership destruction
    /// Destroy an owned value (`drop`).
    ///
    /// Runs drop glue for the value and releases any owned storage.
    Drop {
        /// The value to drop.
        value: ValueReference,
    },

    // address stability
    /// Stabilize one heap value against movement (`pin`).
    ///
    /// While pinned, derived borrowed addresses remain valid across safepoints.
    Pin {
        /// The SSA value to define with the pinned reference.
        destination: ValueReference,
        /// The heap value to pin.
        value: ValueReference,
        /// The result type of the pinned reference.
        result_type: TypeReference,
    },
    /// Release one heap pin (`unpin`).
    Unpin {
        /// The heap value to unpin.
        value: ValueReference,
    },

    // collector protocol
    /// Record a managed reference write for the collector.
    BarrierWrite {
        /// The managed object whose reference range changed.
        object: ValueReference,
        /// The byte offset of the changed reference range.
        offset: ValueReference,
        /// The changed byte length.
        byte_len: ValueReference,
    },

    // atomic memory operations
    /// Load from memory atomically.
    AtomicLoad {
        /// The SSA value to define with the loaded result.
        destination: ValueReference,
        /// The pointer to load from.
        pointer: ValueReference,
        /// The loaded value type.
        result_type: TypeReference,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Store to memory atomically.
    AtomicStore {
        /// The pointer to store to.
        pointer: ValueReference,
        /// The value to store.
        value: ValueReference,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Compare exchange one memory location atomically.
    AtomicCompareExchange {
        /// The SSA value to define with the old value and success flag.
        destination: ValueReference,
        /// The pointer to update.
        pointer: ValueReference,
        /// The expected current value.
        expected: ValueReference,
        /// The replacement value.
        new_value: ValueReference,
        /// Whether the compare exchange is weak.
        is_weak: bool,
        /// The compare exchange access.
        access: CompareExchangeAccess,
    },
    /// Apply one atomic read modify write operation.
    AtomicRmw {
        /// The SSA value to define with the old value.
        destination: ValueReference,
        /// The read modify write operator.
        operator: AtomicRmwOperator,
        /// The pointer to update.
        pointer: ValueReference,
        /// The value argument for the operator.
        value: ValueReference,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Publish one memory fence.
    AtomicFence {
        /// The fence access.
        access: FenceAccess,
    },
    // assumptions and hints
    /// Assume a condition is true (UB if false).
    Assume {
        /// The condition to assume.
        condition: ValueReference,
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
        destination: Option<ValueReference>,
        /// The intrinsic to call.
        intrinsic: Intrinsic,
        /// The arguments to pass.
        arguments: ArgumentSlice,
    },
}

impl Node for Instruction {
    const TYPE: NodeType = NodeType::Instruction;
}

impl Instruction {
    /// Get the destination value defined by this instruction (if any).
    pub fn destination(&self) -> Option<ValueReference> {
        match self {
            Instruction::Error => None,
            Instruction::Const { destination, .. } => Some(*destination),
            Instruction::Binary { destination, .. } => Some(*destination),
            Instruction::Unary { destination, .. } => Some(*destination),
            Instruction::Cast { destination, .. } => Some(*destination),
            Instruction::Select { destination, .. } => Some(*destination),
            Instruction::LocalGet { destination, .. } => Some(*destination),
            Instruction::LocalAddr { destination, .. } => Some(*destination),
            Instruction::LocalSet { .. } => None,
            Instruction::GlobalAddr { destination, .. } => Some(*destination),
            Instruction::FunctionAddr { destination, .. } => Some(*destination),
            Instruction::CallableBind { destination, .. } => Some(*destination),
            Instruction::CallableEnvironment { destination, .. } => Some(*destination),
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
            Instruction::VectorSelect { destination, .. } => Some(*destination),
            Instruction::VectorReduce { destination, .. } => Some(*destination),
            Instruction::VectorCompare { destination, .. } => Some(*destination),
            Instruction::VectorConvert { destination, .. } => Some(*destination),
            Instruction::TensorSplat { destination, .. } => Some(*destination),
            Instruction::TensorLoad { destination, .. } => Some(*destination),
            Instruction::TensorExtract { destination, .. } => Some(*destination),
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
            Instruction::TensorSelect { destination, .. } => Some(*destination),
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
            Instruction::New { destination, .. } => Some(*destination),
            Instruction::NewSlice { destination, .. } => Some(*destination),
            Instruction::RawAlloc { destination, .. } => Some(*destination),
            Instruction::RawFree { .. } => None,
            Instruction::Drop { .. } => None,
            Instruction::Pin { destination, .. } => Some(*destination),
            Instruction::Unpin { .. } => None,
            Instruction::BarrierWrite { .. } => None,
            Instruction::StackAlloc { destination, .. } => Some(*destination),
            Instruction::AtomicLoad { destination, .. } => Some(*destination),
            Instruction::AtomicStore { .. } => None,
            Instruction::AtomicCompareExchange { destination, .. } => Some(*destination),
            Instruction::AtomicRmw { destination, .. } => Some(*destination),
            Instruction::AtomicFence { .. } => None,
            Instruction::Assume { .. } => None,
            Instruction::Intrinsic { destination, .. } => *destination,
        }
    }

    /// Get inline values used by this instruction (excludes externalized arguments).
    ///
    /// For Call, CallVirtual, CallInterface, CallIndirect, and Intrinsic, the arguments are stored externally
    /// in Tree's argument buffer and must be fetched via `Tree::get_arguments()`.
    pub fn uses(&self) -> SmallVec<[ValueReference; 4]> {
        match self {
            Instruction::Error => smallvec![],
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
            Instruction::FunctionAddr { .. } => smallvec![],
            Instruction::CallableBind { environment, .. } => smallvec![*environment],
            Instruction::CallableEnvironment { .. } => smallvec![],
            Instruction::Load { pointer, .. } => smallvec![*pointer],
            Instruction::Store { pointer, value, .. } => smallvec![*pointer, *value],
            Instruction::FieldGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldAddr { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::ElementGet { array, .. } => smallvec![*array],
            Instruction::ElementAddr { array, index, .. } => smallvec![*array, *index],
            Instruction::ElementSet { array, value, .. } => smallvec![*array, *value],
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
            Instruction::VectorSelect {
                mask,
                then_value,
                else_value,
                ..
            } => smallvec![*mask, *then_value, *else_value],
            Instruction::VectorReduce { vector, .. } => smallvec![*vector],
            Instruction::VectorCompare { left, right, .. } => smallvec![*left, *right],
            Instruction::VectorConvert { vector, .. } => smallvec![*vector],
            Instruction::TensorSplat { value, .. } => smallvec![*value],
            Instruction::TensorLoad { view, .. } => smallvec![*view],
            Instruction::TensorExtract { tensor, .. } => smallvec![*tensor],
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
            Instruction::TensorSelect {
                mask,
                then_value,
                else_value,
                ..
            } => smallvec![*mask, *then_value, *else_value],
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
            Instruction::CallIndirect { callee, .. } => smallvec![*callee],
            Instruction::New { .. } => smallvec![],
            Instruction::NewSlice { length, .. } => smallvec![*length],
            Instruction::RawAlloc { .. } => smallvec![],
            Instruction::RawFree { pointer } => smallvec![*pointer],
            Instruction::Drop { value } => smallvec![*value],
            Instruction::Pin { value, .. } => smallvec![*value],
            Instruction::Unpin { value } => smallvec![*value],
            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => smallvec![*object, *offset, *byte_len],
            Instruction::StackAlloc { .. } => smallvec![],
            Instruction::AtomicLoad { pointer, .. } => smallvec![*pointer],
            Instruction::AtomicStore { pointer, value, .. } => smallvec![*pointer, *value],
            Instruction::AtomicCompareExchange {
                pointer,
                expected,
                new_value,
                ..
            } => smallvec![*pointer, *expected, *new_value],
            Instruction::AtomicRmw { pointer, value, .. } => smallvec![*pointer, *value],
            Instruction::AtomicFence { .. } => smallvec![],
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
            Instruction::TensorExtract { indices, .. } => Some(*indices),
            Instruction::TensorStore { indices, .. } => Some(*indices),
            Instruction::TensorReshape { shape, .. } => Some(*shape),
            Instruction::TensorView { arguments, .. } => Some(*arguments),
            Instruction::TensorSlice { arguments, .. } => Some(*arguments),
            Instruction::TensorPad { arguments, .. } => Some(*arguments),
            Instruction::TensorConcat { tensors, .. } => Some(*tensors),
            Instruction::Call { call, .. } => Some(call.arguments),
            Instruction::CallVirtual { call, .. } => Some(call.arguments),
            Instruction::CallInterface { call, .. } => Some(call.arguments),
            Instruction::CallIndirect { call, .. } => Some(call.arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }

    /// Return the dispatch kind for call instructions.
    pub fn call_dispatch_kind(&self) -> Option<CallDispatchKind> {
        match self {
            Instruction::Call { .. } => Some(CallDispatchKind::Direct),
            Instruction::CallVirtual { slot, .. } => {
                Some(CallDispatchKind::Virtual { slot: *slot })
            }
            Instruction::CallInterface { slot, .. } => {
                Some(CallDispatchKind::Interface { slot: *slot })
            }
            Instruction::CallIndirect { .. } => Some(CallDispatchKind::Indirect),
            _ => None,
        }
    }

    /// Return the signature type for call instructions.
    pub fn call_signature(&self) -> Option<TypeReference> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the declared target for call instructions, when known.
    pub fn call_declared_target(&self) -> Option<FunctionReference> {
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

    /// Return the memory effect for call instructions, when present.
    pub fn call_memory_effect(&self) -> Option<&MemoryEffect> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => call.memory_effect.as_ref(),
            _ => None,
        }
    }

    /// Return mutable memory effect storage for call instructions.
    pub fn call_memory_effect_mut(&mut self) -> Option<&mut Option<MemoryEffect>> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&mut call.memory_effect),
            _ => None,
        }
    }

    /// Return the call behavior for call instructions, when present.
    pub fn call_behavior(&self) -> Option<&CallBehavior> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => call.behavior.as_ref(),
            _ => None,
        }
    }

    /// Return mutable call behavior storage for call instructions.
    pub fn call_behavior_mut(&mut self) -> Option<&mut Option<CallBehavior>> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&mut call.behavior),
            _ => None,
        }
    }

    /// Return the allocation size metadata for call instructions, when present.
    pub fn call_allocation_size(&self) -> Option<AllocationSize> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => call.allocation_size,
            _ => None,
        }
    }

    /// Return mutable allocation size storage for call instructions.
    pub fn call_allocation_size_mut(&mut self) -> Option<&mut Option<AllocationSize>> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&mut call.allocation_size),
            _ => None,
        }
    }

    /// Return argument attributes for call instructions.
    pub fn call_argument_attributes(&self) -> Option<&[ArgumentAttribute]> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&call.argument_attributes),
            _ => None,
        }
    }

    /// Return mutable argument attributes for call instructions.
    pub fn call_argument_attributes_mut(&mut self) -> Option<&mut Vec<ArgumentAttribute>> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&mut call.argument_attributes),
            _ => None,
        }
    }

    /// Return the return attribute for call instructions.
    pub fn call_return_attribute(&self) -> Option<&PointerAttribute> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&call.return_attribute),
            _ => None,
        }
    }

    /// Return mutable return attribute storage for call instructions.
    pub fn call_return_attribute_mut(&mut self) -> Option<&mut PointerAttribute> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(&mut call.return_attribute),
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
    /// Convert float to signed integer with saturation.
    FloatToSignedIntSaturating,
    /// Convert float to unsigned integer with saturation.
    FloatToUnsignedIntSaturating,
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
            CastOperator::Bitcast => "cast.bit",
            CastOperator::Truncate => "cast.truncate",
            CastOperator::ZeroExtend => "cast.extend.u",
            CastOperator::SignExtend => "cast.extend.s",
            CastOperator::FloatToSignedInt => "cast.floatToInt.s",
            CastOperator::FloatToUnsignedInt => "cast.floatToInt.u",
            CastOperator::FloatToSignedIntSaturating => "cast.floatToIntSaturating.s",
            CastOperator::FloatToUnsignedIntSaturating => "cast.floatToIntSaturating.u",
            CastOperator::SignedIntToFloat => "cast.intToFloat.s",
            CastOperator::UnsignedIntToFloat => "cast.intToFloat.u",
            CastOperator::FloatTruncate => "cast.floatTruncate",
            CastOperator::FloatExtend => "cast.floatExtend",
            CastOperator::PointerToInt => "cast.pointerToInt",
            CastOperator::IntToPointer => "cast.intToPointer",
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
            "cast.bit" => Ok(CastOperator::Bitcast),
            "cast.truncate" => Ok(CastOperator::Truncate),
            "cast.extend.u" => Ok(CastOperator::ZeroExtend),
            "cast.extend.s" => Ok(CastOperator::SignExtend),
            "cast.floatToInt.s" => Ok(CastOperator::FloatToSignedInt),
            "cast.floatToInt.u" => Ok(CastOperator::FloatToUnsignedInt),
            "cast.floatToIntSaturating.s" => Ok(CastOperator::FloatToSignedIntSaturating),
            "cast.floatToIntSaturating.u" => Ok(CastOperator::FloatToUnsignedIntSaturating),
            "cast.intToFloat.s" => Ok(CastOperator::SignedIntToFloat),
            "cast.intToFloat.u" => Ok(CastOperator::UnsignedIntToFloat),
            "cast.floatTruncate" => Ok(CastOperator::FloatTruncate),
            "cast.floatExtend" => Ok(CastOperator::FloatExtend),
            "cast.pointerToInt" => Ok(CastOperator::PointerToInt),
            "cast.intToPointer" => Ok(CastOperator::IntToPointer),
            _ => Err(()),
        }
    }
}
