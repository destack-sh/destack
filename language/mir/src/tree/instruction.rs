use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    AtomicAccess, AtomicRmwOperator, BinaryOperator, Call, CompareExchangeAccess, Constant,
    CounterId, DispatchSlot, FenceAccess, FunctionId, GlobalId, IndexSlice, Intrinsic, LocalId,
    Node, NodeType, Place, PlaceEffect, Projection, TensorConvertMode, TensorImmediateId,
    TensorIndexReduceOperator, TensorIndexTieBreak, TensorReduceOperator, TensorScatterMode,
    TypeId, UnaryOperator, Value, ValueSlice, VectorConvertMode, VectorReduceOperator,
};

/// Dispatch kind for a call instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallDispatchKind {
    /// Direct function call.
    Direct,
    /// Virtual call through an object dispatch slot.
    Virtual {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Dynamic call through an erased dispatch table slot.
    Dynamic {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Indirect call through a function pointer.
    Indirect,
}

/// Instructions produce SSA values and perform "operations".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    /// Recovered invalid instruction syntax.
    Error,

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
        to_type: TypeId,
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

    // local variables (local.get, local.set, local.address)
    /// Load from a local variable (stack slot).
    LocalGet {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The local variable to load from.
        local: LocalId,
    },
    /// Get the address of a local variable (stack slot).
    LocalAddr {
        /// The SSA value to define with the local address.
        destination: Value,
        /// The local variable to take the address of.
        local: LocalId,
        /// The result type of the address.
        result_type: TypeId,
    },
    /// Store to a local variable (stack slot).
    LocalSet {
        /// The local variable to store to.
        local: LocalId,
        /// The value to store.
        value: Value,
    },

    // global variables (global.address)
    /// Get the address of a mutable global variable.
    /// Returns a raw pointer that can be used with Load/Store.
    GlobalAddr {
        /// The SSA value to define with the pointer.
        destination: Value,
        /// The global variable to get the address of.
        global: GlobalId,
        /// The result type of the address.
        result_type: TypeId,
    },
    /// Get a function pointer for a function (function.address).
    FunctionAddr {
        /// The SSA value to define with the function pointer.
        destination: Value,
        /// The function to take the address of.
        function: FunctionId,
    },
    /// Bind one environment to a function and produce a closure value (closure.bind).
    ClosureBind {
        /// The SSA value to define with the closure value.
        destination: Value,
        /// The function to pair with the environment.
        function: FunctionId,
        /// The environment value to capture in the closure.
        environment: Value,
    },
    /// Load the hidden environment for the current function (closure.environment).
    ClosureEnvironment {
        /// The SSA value to define with the hidden environment pointer.
        destination: Value,
    },

    // memory (pointers)
    /// Load from a pointer (dereference).
    ///
    /// Optional memory access metadata is stored in `Tree::memory_table`.
    Load {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The pointer to load from.
        pointer: Value,
        /// The loaded value type.
        result_type: TypeId,
    },
    /// Store to a pointer (write through pointer).
    ///
    /// Optional memory access metadata is stored in `Tree::memory_table`.
    Store {
        /// The pointer to store to.
        pointer: Value,
        /// The value to store.
        value: Value,
    },

    // aggregate operations (field.get, field.address, field.set, element.get, element.address, element.set)
    /// Extract a field from an aggregate value (field.get).
    FieldGet {
        /// The SSA value to define with the extracted field.
        destination: Value,
        /// The aggregate value to extract from.
        aggregate: Value,
        /// The zero-based field index.
        index: u32,
    },
    /// Get the address of a field from an addressable aggregate (field.address).
    FieldAddr {
        /// The SSA value to define with the field address.
        destination: Value,
        /// The aggregate base to project from.
        aggregate: Value,
        /// The zero-based field index.
        index: u32,
        /// The result type of the address.
        result_type: TypeId,
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
        /// The zero-based element index.
        index: u32,
    },
    /// Get the address of an element from an addressable indexed value (element.address).
    ElementAddr {
        /// The SSA value to define with the element address.
        destination: Value,
        /// The indexed base to project from.
        array: Value,
        /// The index of the element (runtime value).
        index: Value,
        /// The result type of the address.
        result_type: TypeId,
    },
    /// Insert a value into an array element (element.set).
    ElementSet {
        /// The SSA value to define with the new array.
        destination: Value,
        /// The original array value.
        array: Value,
        /// The zero-based element index to update.
        index: u32,
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
        ty: TypeId,
        /// The field values (stored in Tree's argument buffer).
        fields: ValueSlice,
    },
    /// Construct a tuple from element values.
    ///
    /// Elements must be provided in order.
    Tuple {
        /// The SSA value to define with the constructed tuple.
        destination: Value,
        /// The tuple type to construct.
        ty: TypeId,
        /// The element values (stored in Tree's argument buffer).
        elements: ValueSlice,
    },
    /// Construct an array from element values.
    ///
    /// Elements must be provided in index order.
    Array {
        /// The SSA value to define with the constructed array.
        destination: Value,
        /// The array type to construct.
        ty: TypeId,
        /// The element values (stored in Tree's argument buffer).
        elements: ValueSlice,
    },
    /// Construct a non-owning slice descriptor from a contiguous source region.
    Slice {
        /// The SSA value to define with the constructed slice.
        destination: Value,
        /// The source slice value.
        source: Value,
        /// The start index inside the source slice.
        start: Value,
        /// The number of elements in the result.
        length: Value,
        /// The result slice type.
        result_type: TypeId,
    },

    // vector operations
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
        mask: IndexSlice,
    },
    /// Select vector lanes based on a boolean mask.
    VectorSelect {
        /// The SSA value to define with the selected result.
        destination: Value,
        /// The boolean mask vector.
        mask: Value,
        /// The value returned if the mask lane is true.
        then_value: Value,
        /// The value returned if the mask lane is false.
        else_value: Value,
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
    /// Broadcast a scalar to all tensor elements.
    TensorSplat {
        /// The SSA value to define with the tensor result.
        destination: Value,
        /// The scalar value to broadcast.
        value: Value,
    },
    /// Load a tensor element from a tensor reference.
    TensorLoad {
        /// The SSA value to define with the loaded element.
        destination: Value,
        /// The tensor reference to load from.
        view: Value,
        /// The index values (stored in Tree's argument buffer).
        indices: ValueSlice,
    },
    /// Extract a tensor element from a tensor value.
    TensorExtract {
        /// The SSA value to define with the extracted element.
        destination: Value,
        /// The tensor value to extract from.
        tensor: Value,
        /// The index values (stored in Tree's argument buffer).
        indices: ValueSlice,
    },
    /// Store a tensor element into a tensor reference.
    TensorStore {
        /// The tensor reference to store into.
        view: Value,
        /// The index values (stored in Tree's argument buffer).
        indices: ValueSlice,
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
        /// The shape values (stored in Tree's argument buffer).
        shape: ValueSlice,
    },
    /// Broadcast a tensor into a larger shape.
    TensorBroadcast {
        /// The SSA value to define with the broadcasted tensor.
        destination: Value,
        /// The tensor value to broadcast.
        tensor: Value,
        /// The operand dimensions mapped into the result.
        dimensions: IndexSlice,
    },
    /// Permute tensor dimensions.
    TensorTranspose {
        /// The SSA value to define with the transposed tensor.
        destination: Value,
        /// The tensor value to transpose.
        tensor: Value,
        /// The permutation of dimensions.
        permutation: IndexSlice,
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
        /// The view arguments (offsets, sizes, strides) stored in Tree's argument buffer.
        arguments: ValueSlice,
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
        /// The slice arguments (offsets, sizes, strides) stored in Tree's argument buffer.
        arguments: ValueSlice,
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
        /// The padding arguments (low, high, interior) stored in Tree's argument buffer.
        arguments: ValueSlice,
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
        /// The tensor operands stored in Tree's argument buffer.
        tensors: ValueSlice,
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
    /// Select tensor elements based on a boolean mask.
    TensorSelect {
        /// The SSA value to define with the selected tensor.
        destination: Value,
        /// The boolean mask tensor.
        mask: Value,
        /// The tensor returned if the mask element is true.
        then_value: Value,
        /// The tensor returned if the mask element is false.
        else_value: Value,
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
        axes: IndexSlice,
    },
    /// Reduce a tensor along one axis and return selected source indices.
    TensorIndexReduce {
        /// The SSA value to define with the index tensor.
        destination: Value,
        /// The index reduction operator to apply.
        operator: TensorIndexReduceOperator,
        /// The tensor value to reduce.
        tensor: Value,
        /// The axis to reduce.
        axis: u32,
        /// The behavior for equal selected values.
        tie_break: TensorIndexTieBreak,
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
        immediate: TensorImmediateId,
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
        immediate: TensorImmediateId,
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
        immediate: TensorImmediateId,
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
        immediate: TensorImmediateId,
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

    // function calls (call, call.virtual, call.dynamic, call.indirect)
    /// Call a function directly.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function to call.
        function: FunctionId,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Call a virtual method through a virtual dispatch slot.
    CallVirtual {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The receiver value for dispatch.
        receiver: Value,
        /// The class type declaring this dispatch slot.
        class: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Call through a dynamic dispatch table slot.
    CallDynamic {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The receiver value for dispatch.
        receiver: Value,
        /// The dynamic constraint type declaring this dispatch slot.
        constraint: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Call through a function pointer (call.indirect).
    CallIndirect {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The function pointer or closure value to call.
        callee: Value,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },

    // heap allocation
    /// Allocate zeroed typed heap storage (`new.zeroed`).
    ///
    /// The result type decides whether the returned reference is managed or unique.
    NewZeroed {
        /// The SSA value to define with the allocated reference.
        destination: Value,
        /// The type of the struct to allocate.
        layout: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
    },
    /// Allocate uninitialized typed heap storage (`new.uninit`).
    ///
    /// The result is an initialization token that must be completed before publication.
    NewUninit {
        /// The SSA value to define with the initialization token.
        destination: Value,
        /// The type of the struct to allocate.
        layout: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
    },
    /// Complete one initialized heap allocation (`new.complete`).
    NewComplete {
        /// The SSA value to define with the completed allocation.
        destination: Value,
        /// The initialization token to complete.
        value: Value,
        /// The result type of the completed value.
        result_type: TypeId,
    },
    /// Allocate zeroed typed repeated heap storage (`new.slice.zeroed`).
    ///
    /// The result type decides whether the returned slice is managed or unique.
    NewSliceZeroed {
        /// The SSA value to define with the allocated slice.
        destination: Value,
        /// The element type of the repeated storage.
        element: TypeId,
        /// The number of elements (runtime value).
        length: Value,
        /// The result type of the allocation.
        result_type: TypeId,
    },
    /// Allocate uninitialized typed repeated heap storage (`new.slice.uninit`).
    ///
    /// The result is an initialization token that must be completed before publication.
    NewSliceUninit {
        /// The SSA value to define with the initialization token.
        destination: Value,
        /// The element type of the repeated storage.
        element: TypeId,
        /// The number of elements (runtime value).
        length: Value,
        /// The result type of the allocation.
        result_type: TypeId,
    },
    /// Release unique heap storage (`free`).
    ///
    /// This is only valid for unique references after drop elaboration has run.
    Free {
        /// The unique heap reference to free.
        value: Value,
    },

    // frame allocation
    /// Allocate zeroed frame-scoped storage (`frame.alloc.zeroed`).
    ///
    /// The storage is released when the frame exits.
    FrameAllocZeroed {
        /// The SSA value to define with the frame allocation pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
    },
    /// Allocate uninitialized frame-scoped storage (`frame.alloc.uninit`).
    ///
    /// The storage is released when the frame exits.
    FrameAllocUninit {
        /// The SSA value to define with the frame allocation pointer.
        destination: Value,
        /// The type of the value to allocate.
        layout: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
    },

    // ownership end
    /// End ownership of a place (`drop`).
    ///
    /// Drop elaboration consumes this instruction before runtime lowering.
    Drop {
        /// The place to drop.
        place: Place,
    },

    // address stability
    /// Stabilize one heap value against movement (`pin`).
    ///
    /// While pinned, derived borrowed addresses remain valid across safepoints.
    Pin {
        /// The SSA value to define with the pinned reference.
        destination: Value,
        /// The heap value to pin.
        value: Value,
        /// The result type of the pinned reference.
        result_type: TypeId,
    },
    /// Release one heap pin (`unpin`).
    Unpin {
        /// The heap value to unpin.
        value: Value,
    },

    // collector protocol
    /// Record a managed reference write for the collector.
    BarrierWrite {
        /// The managed object whose reference range changed.
        object: Value,
        /// The byte offset of the changed reference range.
        offset: Value,
        /// The changed byte length.
        byte_len: Value,
    },

    // atomic memory operations
    /// Load from memory atomically.
    AtomicLoad {
        /// The SSA value to define with the loaded result.
        destination: Value,
        /// The pointer to load from.
        pointer: Value,
        /// The loaded value type.
        result_type: TypeId,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Store to memory atomically.
    AtomicStore {
        /// The pointer to store to.
        pointer: Value,
        /// The value to store.
        value: Value,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Compare exchange one memory location atomically.
    AtomicCompareExchange {
        /// The SSA value to define with the old value and success flag.
        destination: Value,
        /// The pointer to update.
        pointer: Value,
        /// The expected current value.
        expected: Value,
        /// The replacement value.
        new_value: Value,
        /// Whether the compare exchange is weak.
        is_weak: bool,
        /// The compare exchange access.
        access: CompareExchangeAccess,
    },
    /// Apply one atomic read modify write operation.
    AtomicRmw {
        /// The SSA value to define with the old value.
        destination: Value,
        /// The read modify write operator.
        operator: AtomicRmwOperator,
        /// The pointer to update.
        pointer: Value,
        /// The value argument for the operator.
        value: Value,
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
        condition: Value,
    },

    // profile instrumentation
    /// Increment one profile counter.
    ProfileIncrement {
        /// The counter to increment.
        counter: CounterId,
    },
    /// Record one profiled runtime value.
    ProfileValue {
        /// The counter receiving the sampled value.
        counter: CounterId,
        /// The sampled MIR value.
        value: Value,
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
        arguments: ValueSlice,
    },
}

impl Node for Instruction {
    const TYPE: NodeType = NodeType::Instruction;
}

impl Instruction {
    /// Return the place table effect produced by this instruction.
    pub(crate) fn place_effect(&self) -> Option<PlaceEffect> {
        match self {
            Instruction::LocalAddr {
                destination, local, ..
            } => Some(PlaceEffect::Root {
                value: *destination,
                place: Place::local(*local),
            }),
            Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => Some(PlaceEffect::Root {
                value: *destination,
                place: Place::global(*global),
            }),
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. }
            | Instruction::FrameAllocZeroed { destination, .. }
            | Instruction::FrameAllocUninit { destination, .. }
            | Instruction::ClosureEnvironment { destination } => Some(PlaceEffect::Root {
                value: *destination,
                place: Place::value(*destination),
            }),
            Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } => Some(PlaceEffect::Projection {
                value: *destination,
                base: *aggregate,
                projection: Projection::Field { index: *index },
            }),
            Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => Some(PlaceEffect::Projection {
                value: *destination,
                base: *array,
                projection: Projection::Index { index: *index },
            }),
            Instruction::Slice {
                destination,
                source,
                start,
                length,
                ..
            } => Some(PlaceEffect::Projection {
                value: *destination,
                base: *source,
                projection: Projection::Slice {
                    start: *start,
                    length: *length,
                },
            }),
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::TensorCast {
                destination,
                tensor: argument,
            }
            | Instruction::TensorView {
                destination,
                view: argument,
                ..
            }
            | Instruction::Pin {
                destination,
                value: argument,
                ..
            } => Some(PlaceEffect::Copy {
                value: *destination,
                source: *argument,
            }),
            Instruction::Error
            | Instruction::Const { .. }
            | Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Select { .. }
            | Instruction::LocalGet { .. }
            | Instruction::LocalSet { .. }
            | Instruction::FunctionAddr { .. }
            | Instruction::ClosureBind { .. }
            | Instruction::Load { .. }
            | Instruction::Store { .. }
            | Instruction::FieldGet { .. }
            | Instruction::FieldSet { .. }
            | Instruction::ElementGet { .. }
            | Instruction::ElementSet { .. }
            | Instruction::Struct { .. }
            | Instruction::Tuple { .. }
            | Instruction::Array { .. }
            | Instruction::VectorSplat { .. }
            | Instruction::VectorExtract { .. }
            | Instruction::VectorInsert { .. }
            | Instruction::VectorShuffle { .. }
            | Instruction::VectorSelect { .. }
            | Instruction::VectorReduce { .. }
            | Instruction::VectorCompare { .. }
            | Instruction::VectorConvert { .. }
            | Instruction::TensorSplat { .. }
            | Instruction::TensorLoad { .. }
            | Instruction::TensorExtract { .. }
            | Instruction::TensorStore { .. }
            | Instruction::TensorFill { .. }
            | Instruction::TensorCopy { .. }
            | Instruction::TensorReshape { .. }
            | Instruction::TensorBroadcast { .. }
            | Instruction::TensorTranspose { .. }
            | Instruction::TensorSlice { .. }
            | Instruction::TensorPad { .. }
            | Instruction::TensorConcat { .. }
            | Instruction::TensorCompare { .. }
            | Instruction::TensorSelect { .. }
            | Instruction::TensorReduce { .. }
            | Instruction::TensorIndexReduce { .. }
            | Instruction::TensorDot { .. }
            | Instruction::TensorConvolution { .. }
            | Instruction::TensorGather { .. }
            | Instruction::TensorScatter { .. }
            | Instruction::TensorConvert { .. }
            | Instruction::Call { .. }
            | Instruction::CallVirtual { .. }
            | Instruction::CallDynamic { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::Free { .. }
            | Instruction::Drop { .. }
            | Instruction::Unpin { .. }
            | Instruction::BarrierWrite { .. }
            | Instruction::AtomicLoad { .. }
            | Instruction::AtomicStore { .. }
            | Instruction::AtomicCompareExchange { .. }
            | Instruction::AtomicRmw { .. }
            | Instruction::AtomicFence { .. }
            | Instruction::Assume { .. }
            | Instruction::ProfileIncrement { .. }
            | Instruction::ProfileValue { .. }
            | Instruction::Intrinsic { .. } => None,
        }
    }

    /// Get the destination value defined by this instruction (if any).
    pub fn destination(&self) -> Option<Value> {
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
            Instruction::ClosureBind { destination, .. } => Some(*destination),
            Instruction::ClosureEnvironment { destination, .. } => Some(*destination),
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
            Instruction::Slice { destination, .. } => Some(*destination),
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
            Instruction::TensorIndexReduce { destination, .. } => Some(*destination),
            Instruction::TensorDot { destination, .. } => Some(*destination),
            Instruction::TensorConvolution { destination, .. } => Some(*destination),
            Instruction::TensorGather { destination, .. } => Some(*destination),
            Instruction::TensorScatter { destination, .. } => Some(*destination),
            Instruction::TensorConvert { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::CallVirtual { destination, .. } => *destination,
            Instruction::CallDynamic { destination, .. } => *destination,
            Instruction::CallIndirect { destination, .. } => *destination,
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. }
            | Instruction::FrameAllocZeroed { destination, .. }
            | Instruction::FrameAllocUninit { destination, .. } => Some(*destination),
            Instruction::Free { .. } => None,
            Instruction::Drop { .. } => None,
            Instruction::Pin { destination, .. } => Some(*destination),
            Instruction::Unpin { .. } => None,
            Instruction::BarrierWrite { .. } => None,
            Instruction::AtomicLoad { destination, .. } => Some(*destination),
            Instruction::AtomicStore { .. } => None,
            Instruction::AtomicCompareExchange { destination, .. } => Some(*destination),
            Instruction::AtomicRmw { destination, .. } => Some(*destination),
            Instruction::AtomicFence { .. } => None,
            Instruction::Assume { .. } => None,
            Instruction::ProfileIncrement { .. } => None,
            Instruction::ProfileValue { .. } => None,
            Instruction::Intrinsic { destination, .. } => *destination,
        }
    }

    /// Get inline values used by this instruction (excludes externalized arguments).
    ///
    /// For Call, CallVirtual, CallDynamic, CallIndirect, and Intrinsic, the arguments are stored externally
    /// in Tree's argument buffer and must be fetched via `Tree::get_values()`.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
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
            Instruction::ClosureBind { environment, .. } => smallvec![*environment],
            Instruction::ClosureEnvironment { .. } => smallvec![],
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
            Instruction::Slice {
                source,
                start,
                length,
                ..
            } => smallvec![*source, *start, *length],
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
            Instruction::TensorIndexReduce { tensor, .. } => smallvec![*tensor],
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
            Instruction::CallDynamic { receiver, .. } => smallvec![*receiver],
            Instruction::CallIndirect { callee, .. } => smallvec![*callee],
            Instruction::NewZeroed { .. } | Instruction::NewUninit { .. } => smallvec![],
            Instruction::NewComplete { value, .. } => smallvec![*value],
            Instruction::NewSliceZeroed { length, .. }
            | Instruction::NewSliceUninit { length, .. } => smallvec![*length],
            Instruction::Free { value } => smallvec![*value],
            Instruction::Drop { place } => place.values(),
            Instruction::Pin { value, .. } => smallvec![*value],
            Instruction::Unpin { value } => smallvec![*value],
            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => smallvec![*object, *offset, *byte_len],
            Instruction::FrameAllocZeroed { .. } | Instruction::FrameAllocUninit { .. } => {
                smallvec![]
            }
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
            Instruction::ProfileIncrement { .. } => smallvec![],
            Instruction::ProfileValue { value, .. } => smallvec![*value],
            // Arguments stored externally - return empty
            Instruction::Intrinsic { .. } => smallvec![],
        }
    }

    /// Get the argument slice for instructions that have externalized arguments.
    ///
    /// Returns `Some(ValueSlice)` for Struct, Tuple, Array, Call, CallVirtual, CallDynamic,
    /// CallIndirect, Intrinsic, and tensor instructions that externalize value lists.
    /// Returns `None` for all other instructions.
    pub fn argument_slice(&self) -> Option<ValueSlice> {
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
            Instruction::CallDynamic { call, .. } => Some(call.arguments),
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
            Instruction::CallDynamic { slot, .. } => {
                Some(CallDispatchKind::Dynamic { slot: *slot })
            }
            Instruction::CallIndirect { .. } => Some(CallDispatchKind::Indirect),
            _ => None,
        }
    }

    /// Return the signature type for call instructions.
    pub fn call_signature(&self) -> Option<TypeId> {
        match self {
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallDynamic { call, .. }
            | Instruction::CallIndirect { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the direct target for call instructions.
    pub fn call_direct_target(&self) -> Option<FunctionId> {
        match self {
            Instruction::Call { function, .. } => Some(*function),
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
    /// Convert float to another format at the same width.
    FloatConvert,
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
            CastOperator::FloatConvert => "cast.floatConvert",
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
            "cast.floatConvert" => Ok(CastOperator::FloatConvert),
            "cast.pointerToInt" => Ok(CastOperator::PointerToInt),
            "cast.intToPointer" => Ok(CastOperator::IntToPointer),
            _ => Err(()),
        }
    }
}
