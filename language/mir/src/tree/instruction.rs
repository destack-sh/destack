use destack_serde::Reflect;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    AtomicAccess, AtomicRmwOperator, BinaryOperator, Call, CallDispatch, CompareExchangeAccess,
    Constant, CounterId, FenceAccess, FunctionId, GlobalId, IndexSlice, Intrinsic, LocalId, Node,
    NodeType, TensorConvertMode, TensorImmediateId, TensorIndexReduceOperator, TensorIndexTieBreak,
    TensorReduceOperator, TensorScatterMode, Tree, TypeId, UnaryOperator, Value, ValueSlice,
    VectorConvertMode, VectorReduceOperator,
};

/// Instructions produce SSA values and perform "operations".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Bind one environment to a function and produce a function value (function.bind).
    FunctionBind {
        /// The SSA value to define with the function value.
        destination: Value,
        /// The function to pair with the environment.
        function: FunctionId,
        /// The environment value to capture.
        environment: Value,
    },
    /// Project the function pointer from one function value (function.pointer).
    FunctionPointer {
        /// The SSA value to define with the function pointer.
        destination: Value,
        /// The function value to project.
        function: Value,
    },
    /// Project the environment from one function value (function.environment).
    FunctionEnvironment {
        /// The SSA value to define with the environment.
        destination: Value,
        /// The function value to project.
        function: Value,
    },
    /// Load the hidden environment for the current function (function.environment.current).
    FunctionEnvironmentCurrent {
        /// The SSA value to define with the hidden environment pointer.
        destination: Value,
    },

    // memory (pointers)
    /// Load from a pointer (dereference).
    Load {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The pointer to load from.
        pointer: Value,
        /// The loaded value type.
        result_type: TypeId,
    },
    /// Store to a pointer (write through pointer).
    Store {
        /// The pointer to store to.
        pointer: Value,
        /// The value to store.
        value: Value,
    },

    // aggregate construction
    /// Construct an aggregate from values in logical slot order.
    Aggregate {
        /// The SSA value to define with the constructed aggregate.
        destination: Value,
        /// The aggregate values stored in the tree's value buffer.
        values: ValueSlice,
    },

    // aggregate projection
    /// Extract one structural field from an aggregate value.
    FieldGet {
        /// The SSA value to define with the extracted field.
        destination: Value,
        /// The aggregate value to extract from.
        aggregate: Value,
        /// The zero-based logical field.
        field: u32,
    },
    /// Replace one structural field in an aggregate value.
    FieldSet {
        /// The SSA value to define with the new aggregate.
        destination: Value,
        /// The original aggregate value.
        aggregate: Value,
        /// The zero-based logical field to update.
        field: u32,
        /// The value to insert at the field.
        value: Value,
    },
    /// Get the address of one structural field in an addressable aggregate.
    FieldAddr {
        /// The SSA value to define with the field address.
        destination: Value,
        /// The aggregate base to project from.
        aggregate: Value,
        /// The zero-based logical field.
        field: u32,
        /// The result type of the address.
        result_type: TypeId,
    },
    /// Extract one statically selected fixed-array element.
    ElementGet {
        /// The SSA value to define with the extracted element.
        destination: Value,
        /// The fixed-array aggregate to extract from.
        aggregate: Value,
        /// The zero-based element index.
        index: u32,
    },
    /// Insert one value into a statically selected fixed-array element.
    ElementSet {
        /// The SSA value to define with the updated fixed array.
        destination: Value,
        /// The original fixed-array aggregate.
        aggregate: Value,
        /// The zero-based element index.
        index: u32,
        /// The value to insert.
        value: Value,
    },
    /// Get the address of an element from an addressable indexed value (element.address).
    ElementAddr {
        /// The SSA value to define with the element address.
        destination: Value,
        /// The indexed base to project from.
        base: Value,
        /// The index of the element (runtime value).
        index: Value,
        /// The result type of the address.
        result_type: TypeId,
    },

    // variant construction and projection
    /// Construct a variant value from one case payload.
    VariantNew {
        /// The SSA value to define with the constructed variant.
        destination: Value,
        /// The zero-based case index.
        case: u32,
        /// The case payload, or None for payloadless cases.
        payload: Option<Value>,
        /// The result variant type.
        result_type: TypeId,
    },
    /// Read the discriminant of a variant value.
    VariantTag {
        /// The SSA value to define with the discriminant.
        destination: Value,
        /// The variant value whose discriminant is read.
        variant: Value,
    },
    /// Extract the payload of one statically selected variant case.
    VariantPayload {
        /// The SSA value to define with the extracted payload.
        destination: Value,
        /// The variant value to extract from.
        variant: Value,
        /// The zero-based case index.
        case: u32,
    },

    // slice descriptors
    /// Form a non-owning slice view over a contiguous source region.
    SliceView {
        /// The SSA value to define with the slice view.
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
    /// Read the runtime length from a slice descriptor.
    SliceLength {
        /// The SSA value to define with the length.
        destination: Value,
        /// The slice value whose length is read.
        slice: Value,
    },

    // dynamic values
    /// Bind a typed managed payload to its concrete runtime type.
    DynamicBind {
        /// The SSA value to define with the dynamic value.
        destination: Value,
        /// The typed local managed payload.
        payload: Value,
        /// The concrete payload type.
        concrete: TypeId,
    },
    /// Read the erased payload from a dynamic value.
    DynamicPayload {
        /// The SSA value to define with the payload.
        destination: Value,
        /// The dynamic value whose payload is read.
        dynamic: Value,
        /// The result type of the payload value.
        result_type: TypeId,
    },
    /// Read the concrete type id from a dynamic value.
    DynamicType {
        /// The SSA value to define with the type id.
        destination: Value,
        /// The dynamic value whose concrete type is read.
        dynamic: Value,
    },
    /// Read one dispatch slot through a dynamic value's concrete table.
    DynamicRead {
        /// The SSA value to define with the slot value.
        destination: Value,
        /// The dynamic value whose entry is read.
        dynamic: Value,
        /// The zero-based slot in the constraint's dynamic shape.
        slot: u32,
        /// The result type of the slot value.
        result_type: TypeId,
    },
    /// Find one named entry through a dynamic value's concrete table.
    DynamicFind {
        /// The SSA value to define with the optional entry value.
        destination: Value,
        /// The dynamic value whose entries are searched.
        dynamic: Value,
        /// The string key value naming the entry.
        key: Value,
        /// The result type carrying the found value or undefined.
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

    // function calls
    /// Call one callable target without a local unwind continuation.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The call operation.
        call: Call,
    },
    /// Drop one value.
    Drop {
        /// The value to drop.
        value: Value,
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
    /// Sample one runtime value.
    ProfileSample {
        /// The counter receiving the sampled value.
        counter: CounterId,
        /// The sampled MIR value.
        value: Value,
    },

    // debug control
    /// Debugger breakpoint.
    Breakpoint,

    // intrinsics
    /// Call a machine intrinsic.
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
            Instruction::FunctionBind { destination, .. } => Some(*destination),
            Instruction::FunctionEnvironment { destination, .. } => Some(*destination),
            Instruction::FunctionPointer { destination, .. } => Some(*destination),
            Instruction::FunctionEnvironmentCurrent { destination, .. } => Some(*destination),
            Instruction::Load { destination, .. } => Some(*destination),
            Instruction::Store { .. } => None,
            Instruction::Aggregate { destination, .. } => Some(*destination),
            Instruction::FieldGet { destination, .. } => Some(*destination),
            Instruction::FieldSet { destination, .. } => Some(*destination),
            Instruction::FieldAddr { destination, .. } => Some(*destination),
            Instruction::ElementGet { destination, .. } => Some(*destination),
            Instruction::ElementSet { destination, .. } => Some(*destination),
            Instruction::ElementAddr { destination, .. } => Some(*destination),
            Instruction::VariantNew { destination, .. } => Some(*destination),
            Instruction::VariantTag { destination, .. } => Some(*destination),
            Instruction::VariantPayload { destination, .. } => Some(*destination),
            Instruction::SliceView { destination, .. } => Some(*destination),
            Instruction::SliceLength { destination, .. } => Some(*destination),
            Instruction::DynamicBind { destination, .. } => Some(*destination),
            Instruction::DynamicPayload { destination, .. } => Some(*destination),
            Instruction::DynamicType { destination, .. } => Some(*destination),
            Instruction::DynamicRead { destination, .. } => Some(*destination),
            Instruction::DynamicFind { destination, .. } => Some(*destination),
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
            Instruction::Drop { .. } => None,
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. } => Some(*destination),
            Instruction::Free { .. } => None,
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
            Instruction::ProfileSample { .. } => None,
            Instruction::Breakpoint => None,
            Instruction::Intrinsic { destination, .. } => *destination,
        }
    }

    /// Get inline values used by this instruction (excludes externalized arguments).
    ///
    /// Call and intrinsic arguments are stored externally in the tree's value buffer.
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
            Instruction::FunctionBind { environment, .. } => smallvec![*environment],
            Instruction::FunctionPointer { function, .. }
            | Instruction::FunctionEnvironment { function, .. } => smallvec![*function],
            Instruction::FunctionEnvironmentCurrent { .. } => smallvec![],
            Instruction::Load { pointer, .. } => smallvec![*pointer],
            Instruction::Store { pointer, value, .. } => smallvec![*pointer, *value],
            // arguments stored externally
            Instruction::Aggregate { .. } => smallvec![],
            Instruction::FieldGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::FieldAddr { aggregate, .. } => smallvec![*aggregate],
            Instruction::ElementGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::ElementSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::ElementAddr { base, index, .. } => smallvec![*base, *index],
            Instruction::VariantNew { payload, .. } => {
                payload.iter().copied().collect::<SmallVec<[Value; 4]>>()
            }
            Instruction::VariantTag { variant, .. } => smallvec![*variant],
            Instruction::VariantPayload { variant, .. } => smallvec![*variant],
            Instruction::SliceView {
                source,
                start,
                length,
                ..
            } => smallvec![*source, *start, *length],
            Instruction::SliceLength { slice, .. } => smallvec![*slice],
            Instruction::DynamicBind { payload, .. } => smallvec![*payload],
            Instruction::DynamicPayload { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicType { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicRead { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicFind { dynamic, key, .. } => smallvec![*dynamic, *key],
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
            Instruction::Call { call, .. } => call.callee.uses().into_iter().collect(),
            Instruction::Drop { value } => smallvec![*value],
            Instruction::NewZeroed { .. } | Instruction::NewUninit { .. } => smallvec![],
            Instruction::NewComplete { value, .. } => smallvec![*value],
            Instruction::NewSliceZeroed { length, .. }
            | Instruction::NewSliceUninit { length, .. } => smallvec![*length],
            Instruction::Free { value } => smallvec![*value],
            Instruction::Pin { value, .. } => smallvec![*value],
            Instruction::Unpin { value } => smallvec![*value],
            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => smallvec![*object, *offset, *byte_len],
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
            Instruction::ProfileSample { value, .. } => smallvec![*value],
            Instruction::Breakpoint => smallvec![],
            Instruction::Intrinsic { .. } => smallvec![],
        }
    }

    /// Return every value read by this instruction.
    pub fn reads(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        let mut values = self.uses().into_iter().collect::<SmallVec<[Value; 8]>>();

        if let Some(arguments) = self.argument_slice() {
            values.extend(tree.get_values(arguments).iter().copied());
        }

        values
    }

    /// Return values consumed by this instruction.
    pub fn consumes(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        match self {
            Instruction::LocalSet { value, .. }
            | Instruction::Store { value, .. }
            | Instruction::NewComplete { value, .. }
            | Instruction::TensorStore { value, .. }
            | Instruction::TensorFill { value, .. }
            | Instruction::Free { value } => smallvec![*value],
            Instruction::AtomicStore { value, .. } | Instruction::AtomicRmw { value, .. } => {
                smallvec![*value]
            }
            Instruction::AtomicCompareExchange {
                expected,
                new_value,
                ..
            } => smallvec![*expected, *new_value],
            Instruction::FieldSet {
                aggregate, value, ..
            }
            | Instruction::ElementSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::VariantNew { payload, .. } => {
                payload.iter().copied().collect::<SmallVec<[Value; 8]>>()
            }
            Instruction::FunctionBind { environment, .. }
            | Instruction::VectorSplat {
                value: environment, ..
            }
            | Instruction::TensorSplat {
                value: environment, ..
            } => smallvec![*environment],
            Instruction::FunctionPointer { .. }
            | Instruction::FunctionEnvironment { .. }
            | Instruction::FunctionEnvironmentCurrent { .. } => smallvec![],
            Instruction::VectorInsert { vector, value, .. }
            | Instruction::TensorPad {
                tensor: vector,
                value,
                ..
            } => smallvec![*vector, *value],
            Instruction::TensorExtract { tensor, .. }
            | Instruction::TensorReshape { tensor, .. }
            | Instruction::TensorBroadcast { tensor, .. }
            | Instruction::TensorTranspose { tensor, .. }
            | Instruction::TensorCast { tensor, .. }
            | Instruction::TensorSlice { tensor, .. }
            | Instruction::TensorReduce { tensor, .. }
            | Instruction::TensorIndexReduce { tensor, .. }
            | Instruction::TensorConvert { tensor, .. } => smallvec![*tensor],
            Instruction::TensorDot { left, right, .. }
            | Instruction::TensorCompare { left, right, .. } => smallvec![*left, *right],
            Instruction::TensorConvolution { input, kernel, .. } => {
                smallvec![*input, *kernel]
            }
            Instruction::TensorGather {
                operand, indices, ..
            } => smallvec![*operand, *indices],
            Instruction::TensorScatter {
                operand,
                indices,
                updates,
                ..
            } => smallvec![*operand, *indices, *updates],
            Instruction::Aggregate { .. } | Instruction::TensorConcat { .. } => {
                self.argument_slice_values(tree)
            }
            Instruction::Call { call, .. } => {
                let mut values = call
                    .callee
                    .uses()
                    .into_iter()
                    .collect::<SmallVec<[Value; 8]>>();
                values.extend(tree.get_values(call.arguments).iter().copied());

                values
            }
            Instruction::Drop { value } => smallvec![*value],
            Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                let arguments = tree.get_values(*arguments);
                let mut values = SmallVec::<[Value; 8]>::new();

                for &index in intrinsic.consumed_arguments() {
                    let Some(&value) = arguments.get(index as usize) else {
                        continue;
                    };

                    values.push(value);
                }

                values
            }
            _ => smallvec![],
        }
    }

    /// Return externally stored values for this instruction.
    fn argument_slice_values(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        let Some(arguments) = self.argument_slice() else {
            return smallvec![];
        };

        tree.get_values(arguments).iter().copied().collect()
    }

    /// Get the argument slice for instructions that have externalized arguments.
    ///
    /// Returns `Some(ValueSlice)` for instructions that externalize value lists.
    /// Returns `None` for all other instructions.
    pub fn argument_slice(&self) -> Option<ValueSlice> {
        match self {
            Instruction::Aggregate { values, .. } => Some(*values),
            Instruction::TensorLoad { indices, .. } => Some(*indices),
            Instruction::TensorExtract { indices, .. } => Some(*indices),
            Instruction::TensorStore { indices, .. } => Some(*indices),
            Instruction::TensorReshape { shape, .. } => Some(*shape),
            Instruction::TensorView { arguments, .. } => Some(*arguments),
            Instruction::TensorSlice { arguments, .. } => Some(*arguments),
            Instruction::TensorPad { arguments, .. } => Some(*arguments),
            Instruction::TensorConcat { tensors, .. } => Some(*tensors),
            Instruction::Call { call, .. } => Some(call.arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }

    /// Return the dispatch for a call instruction.
    pub fn call_dispatch(&self) -> Option<CallDispatch> {
        match self {
            Instruction::Call { call, .. } => Some(call.callee.dispatch()),
            _ => None,
        }
    }

    /// Return the signature type for call instructions.
    pub fn call_signature(&self) -> Option<TypeId> {
        match self {
            Instruction::Call { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the direct target for call instructions.
    pub fn call_direct_target(&self) -> Option<FunctionId> {
        match self {
            Instruction::Call { call, .. } => call.callee.function(),
            _ => None,
        }
    }
}

/// Kind of type cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CastOperator {
    /// Bitcast (reinterpret bits, same size).
    Bitcast,
    /// Truncate integer to smaller width.
    Truncate,
    /// Clamp integer into the destination range.
    Saturate,
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
            CastOperator::Saturate => "cast.saturate",
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
            "cast.saturate" => Ok(CastOperator::Saturate),
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
