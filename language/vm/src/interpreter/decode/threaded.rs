#![allow(elided_lifetimes_in_paths)]

use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

use destack_mir as mir;

use super::super::state::{Frame, InterpreterContext};
use crate::diagnostic::{Error, RuntimeResult};
use destack_heap::{Heap, LayoutId, ReferenceMap, ReferenceMeta, Value};

/// Handler function for threaded dispatch.
///
/// Takes state, current block's instructions, and program counter.
/// Uses `become` to tail-call next handler, or returns `ControlFlow` for special cases.
pub type ThreadedHandler = for<'ctx, 'iso> fn(
    &mut ThreadedState<'ctx, 'iso>,
    &[ThreadedInstruction],
    usize,
) -> ControlFlow;

/// Argument range within the threaded function argument pool.
#[derive(Clone, Copy, Debug)]
pub struct ArgumentRange {
    /// Start offset into the argument pool.
    pub start: u32,
    /// Number of arguments in the range.
    pub len: u32,
    /// Whether the arguments are contiguous SSA ids.
    pub is_contiguous: bool,
    /// First SSA value id when contiguous.
    pub contiguous_start: u32,
}

impl ArgumentRange {
    /// Create an empty argument range.
    pub const fn empty() -> Self {
        Self {
            start: 0,
            len: 0,
            is_contiguous: false,
            contiguous_start: 0,
        }
    }

    /// Slice arguments from the pool for this range.
    #[inline(always)]
    pub fn slice<'a>(&self, pool: &'a [mir::Value]) -> &'a [mir::Value] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "argument pool out of bounds for range"
        );

        // return argument slice
        &pool[start..start + len]
    }

    /// Get the contiguous range for fast copying when available.
    #[inline(always)]
    pub fn contiguous_range(&self) -> Option<(u32, usize)> {
        if self.is_contiguous && self.len > 0 {
            return Some((self.contiguous_start, self.len as usize));
        }

        None
    }
}

/// Switch case range within the threaded function switch pool.
#[derive(Clone, Copy, Debug)]
pub struct SwitchRange {
    /// Start offset into the switch case pool.
    pub start: u32,
    /// Number of cases in the range.
    pub len: u32,
}

impl SwitchRange {
    /// Create an empty switch range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice cases from the pool for this range.
    #[inline(always)]
    pub fn slice<'a>(&self, pool: &'a [SwitchCase]) -> &'a [SwitchCase] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "switch case pool out of bounds for range"
        );

        // return case slice
        &pool[start..start + len]
    }
}

/// Copy range within the threaded function copy pool.
#[derive(Clone, Copy, Debug)]
pub struct CopyRange {
    /// Start offset into the copy pool.
    pub start: u32,
    /// Number of pairs in the range.
    pub len: u32,
    /// Whether the copies are contiguous pairs.
    pub is_contiguous: bool,
    /// First source id when contiguous.
    pub contiguous_src: u32,
    /// First destination id when contiguous.
    pub contiguous_dest: u32,
}

impl CopyRange {
    /// Create an empty copy range.
    pub const fn empty() -> Self {
        Self {
            start: 0,
            len: 0,
            is_contiguous: false,
            contiguous_src: 0,
            contiguous_dest: 0,
        }
    }

    /// Slice pairs from the pool for this range.
    #[inline(always)]
    pub fn slice<'a>(&self, pool: &'a [CopyPair]) -> &'a [CopyPair] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "copy pool out of bounds for range"
        );

        // return copy slice
        &pool[start..start + len]
    }

    /// Get the contiguous copy plan when available.
    #[inline(always)]
    pub fn contiguous_plan(&self) -> Option<(u32, u32, usize)> {
        if self.is_contiguous && self.len > 0 {
            return Some((self.contiguous_src, self.contiguous_dest, self.len as usize));
        }

        None
    }
}

/// Sentinel value id used for optional destinations.
pub(crate) const INVALID_VALUE_ID: u32 = u32::MAX;

/// Sentinel function index used for missing threaded entries.
pub(crate) const INVALID_FUNCTION_INDEX: u32 = u32::MAX;

/// Sentinel field count for unknown aggregate layouts.
pub(crate) const UNKNOWN_FIELD_COUNT: u32 = u32::MAX;

/// Sentinel array length for unknown layouts.
pub(crate) const UNKNOWN_ARRAY_LENGTH: u64 = u64::MAX;

/// Sentinel slot count for unknown allocation layouts.
pub(crate) const UNKNOWN_SLOT_COUNT: u32 = u32::MAX;

/// Pack an optional SSA value into a sentinel encoding.
pub(crate) fn pack_optional_value(value: Option<mir::Value>) -> mir::Value {
    value.unwrap_or(mir::Value(INVALID_VALUE_ID))
}

/// Check if a packed SSA value is the sentinel.
pub(crate) fn is_invalid_value(value: mir::Value) -> bool {
    value.0 == INVALID_VALUE_ID
}

/// Control flow actions that exit the tail-call chain.
#[derive(Debug)]
pub enum ControlFlow {
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: u32,
        /// Copy plan for block parameters.
        copies: CopyRange,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: u32,
        /// Threaded function index when available.
        callee_index: u32,
        /// Destination for return value.
        destination: mir::Value,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional closure environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: u32,
        /// Threaded function index when available.
        callee_index: u32,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional closure environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Value,
        /// Resume block index.
        resume_block: u32,
        /// Copy plan for resume arguments.
        resume_copies: CopyRange,
        /// Destination for the resumed value.
        resume_value: mir::Value,
    },
    /// Return from current function.
    Return(Value),
    /// Runtime error.
    Error(Error),
}

/// Pre-decoded instruction with handler pointer.
#[derive(Clone)]
pub struct ThreadedInstruction {
    /// Handler function.
    pub handler: ThreadedHandler,
    /// Decoded data.
    pub data: ThreadedInstructionData,
}

/// Constant value payload stored in threaded instructions.
#[derive(Clone, Debug)]
pub enum ConstValue {
    /// Pre-decoded constant value.
    Value(Value),
}

/// Decoded instruction data.
#[derive(Clone, Debug)]
pub enum ThreadedInstructionData {
    /// Load constant.
    Const { dest: mir::Value, value: ConstValue },

    /// Binary operation.
    Binary {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    },

    /// Elementwise binary operation on vector or tensor values.
    BinaryElementwise {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        result_type: mir::LocalNodeId<mir::Type>,
    },

    /// Specialized binary operation (operator baked into handler).
    BinarySpecialized {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
    },

    /// Binary operation with constant right operand.
    BinaryConstRight {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right_const: Value,
    },

    /// Specialized binary with constant right (operator baked into handler).
    BinaryConstRightSpecialized {
        dest: mir::Value,
        left: mir::Value,
        right_const: Value,
    },

    /// Unary operation.
    Unary {
        dest: mir::Value,
        op: mir::UnaryOperator,
        arg: mir::Value,
    },

    /// Elementwise unary operation on vector or tensor values.
    UnaryElementwise {
        dest: mir::Value,
        op: mir::UnaryOperator,
        arg: mir::Value,
        result_type: mir::LocalNodeId<mir::Type>,
    },

    /// Type cast.
    Cast {
        dest: mir::Value,
        op: mir::CastOperator,
        arg: mir::Value,
        to_type: u32,
    },

    /// Conditional select.
    Select {
        dest: mir::Value,
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    },

    /// Function call.
    Call {
        dest: mir::Value,
        function: u32,
        callee_index: u32,
        arguments: ArgumentRange,
        copies: CopyRange,
    },

    /// Virtual method call.
    CallVirtual {
        dest: mir::Value,
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface method call.
    CallInterface {
        dest: mir::Value,
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Indirect function call.
    CallIndirect {
        dest: mir::Value,
        callee: mir::Value,
        env: Option<mir::Value>,
        arguments: ArgumentRange,
        cached_function: Cell<Option<u32>>,
        cached_index: Cell<Option<u32>>,
    },

    /// Load local variable.
    LocalGet { dest: mir::Value, local: u32 },

    /// Get local address.
    LocalAddr {
        dest: mir::Value,
        local: u32,
        reference: ReferenceMeta,
    },

    /// Store local variable.
    LocalSet { local: u32, value: mir::Value },

    /// Get global address.
    GlobalAddr {
        dest: mir::Value,
        global: u32,
        reference: ReferenceMeta,
    },

    /// Load global constant.
    GlobalConst { dest: mir::Value, global: u32 },

    /// Get a function pointer.
    FunctionAddr { dest: mir::Value, function: u32 },
    /// Load the closure environment pointer.
    FunctionEnv { dest: mir::Value },
    /// Create a null reference value.

    /// Fused global address + load.
    GlobalLoad { dest: mir::Value, global: u32 },

    /// Fused global address + store.
    GlobalStore {
        global: u32,
        value: mir::Value,
        reference: ReferenceMeta,
    },

    /// Load from pointer.
    Load {
        dest: mir::Value,
        pointer: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Store to pointer.
    Store {
        pointer: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Get struct/tuple field.
    FieldGet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        field_count: u32,
    },

    /// Get struct/tuple field address.
    FieldAddr {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        reference: ReferenceMeta,
        field_count: u32,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Load a field through field.addr + load.
    FieldLoad {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        field_count: u32,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Set struct/tuple field.
    FieldSet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    },

    /// Store a field through field.addr + store.
    FieldStore {
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
        reference: ReferenceMeta,
        field_count: u32,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Get array element.
    ElementGet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
    },

    /// Get array element address.
    ElementAddr {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Load an element through element.addr + load.
    ElementLoad {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        array_length: u64,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Set array element.
    ElementSet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
    },

    /// Construct an aggregate (struct, tuple, or array) from element values.
    Aggregate {
        dest: mir::Value,
        elements: ArgumentRange,
    },

    /// Broadcast a scalar to all vector lanes.
    VectorSplat {
        dest: mir::Value,
        value: mir::Value,
        lanes: u32,
    },

    /// Extract a lane from a vector.
    VectorExtract {
        dest: mir::Value,
        vector: mir::Value,
        index: mir::Value,
    },

    /// Insert a lane into a vector.
    VectorInsert {
        dest: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        value: mir::Value,
    },

    /// Shuffle vector lanes using a constant mask.
    VectorShuffle {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
        mask: Vec<u32>,
    },

    /// Select vector lanes based on a boolean mask.
    VectorSelect {
        dest: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    },

    /// Reduce a vector to a scalar.
    VectorReduce {
        dest: mir::Value,
        operator: mir::VectorReduceOperator,
        vector: mir::Value,
    },

    /// Compare two vectors elementwise.
    VectorCompare {
        dest: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    },

    /// Convert vector element types using an explicit mode.
    VectorConvert {
        dest: mir::Value,
        mode: mir::VectorConvertMode,
        vector: mir::Value,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Load a tensor element from a view.
    TensorLoad {
        dest: mir::Value,
        view: mir::Value,
        indices: ArgumentRange,
        view_type: mir::LocalNodeId<mir::Type>,
    },

    /// Store a tensor element into a view.
    TensorStore {
        view: mir::Value,
        indices: ArgumentRange,
        value: mir::Value,
        view_type: mir::LocalNodeId<mir::Type>,
    },

    /// Fill a tensor reference with a scalar value.
    TensorFill {
        view: mir::Value,
        value: mir::Value,
        view_type: mir::LocalNodeId<mir::Type>,
    },

    /// Copy elements between tensor references.
    TensorCopy {
        target: mir::Value,
        source: mir::Value,
        target_type: mir::LocalNodeId<mir::Type>,
        source_type: mir::LocalNodeId<mir::Type>,
    },

    /// Reshape a tensor into a new shape.
    TensorReshape {
        dest: mir::Value,
        tensor: mir::Value,
        shape: ArgumentRange,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Broadcast a tensor into a larger shape.
    TensorBroadcast {
        dest: mir::Value,
        tensor: mir::Value,
        dimensions: Vec<u32>,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Permute tensor dimensions.
    TensorTranspose {
        dest: mir::Value,
        tensor: mir::Value,
        permutation: Vec<u32>,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Slice a tensor by offsets, sizes, and strides.
    TensorSlice {
        dest: mir::Value,
        tensor: mir::Value,
        arguments: ArgumentRange,
        offsets_count: u16,
        sizes_count: u16,
        strides_count: u16,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Pad a tensor with low, high, and interior padding.
    TensorPad {
        dest: mir::Value,
        tensor: mir::Value,
        arguments: ArgumentRange,
        low_count: u16,
        high_count: u16,
        interior_count: u16,
        value: mir::Value,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Concatenate tensors along a dimension.
    TensorConcat {
        /// The destination value.
        dest: mir::Value,
        /// The input tensor values.
        tensors: ArgumentRange,
        /// The input tensor types.
        tensor_types: Vec<mir::LocalNodeId<mir::Type>>,
        /// The concatenation axis.
        axis: u32,
        /// The destination tensor type.
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Reduce a tensor along axes.
    TensorReduce {
        dest: mir::Value,
        operator: mir::TensorReduceOperator,
        tensor: mir::Value,
        initial: mir::Value,
        axes: Vec<u32>,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Dot product of two tensors.
    TensorDot {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
        dimensions: mir::TensorDotDimensionNumbers,
        left_type: mir::LocalNodeId<mir::Type>,
        right_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Convolution between an input tensor and a kernel tensor.
    TensorConvolution {
        dest: mir::Value,
        input: mir::Value,
        kernel: mir::Value,
        dimensions: mir::TensorConvolutionDimensionNumbers,
        window: mir::TensorConvolutionWindow,
        feature_group_count: u32,
        batch_group_count: u32,
        input_type: mir::LocalNodeId<mir::Type>,
        kernel_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Gather slices from a tensor based on indices.
    TensorGather {
        dest: mir::Value,
        operand: mir::Value,
        indices: mir::Value,
        dimensions: mir::TensorGatherDimensionNumbers,
        slice_sizes: Vec<u32>,
        operand_type: mir::LocalNodeId<mir::Type>,
        indices_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Scatter updates into a tensor based on indices.
    TensorScatter {
        dest: mir::Value,
        operand: mir::Value,
        indices: mir::Value,
        updates: mir::Value,
        dimensions: mir::TensorScatterDimensionNumbers,
        mode: mir::TensorScatterMode,
        operand_type: mir::LocalNodeId<mir::Type>,
        indices_type: mir::LocalNodeId<mir::Type>,
        updates_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Compare two tensors elementwise.
    TensorCompare {
        dest: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        left_type: mir::LocalNodeId<mir::Type>,
        right_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Select tensor elements based on a boolean mask.
    TensorSelect {
        dest: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Convert a tensor element type.
    TensorConvert {
        dest: mir::Value,
        mode: mir::TensorConvertMode,
        tensor: mir::Value,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Refine a tensor type without changing its contents.
    TensorCast {
        dest: mir::Value,
        tensor: mir::Value,
    },

    /// Create a view into a tensor reference.
    TensorView {
        dest: mir::Value,
        view: mir::Value,
        arguments: ArgumentRange,
        offsets_count: u16,
        sizes_count: u16,
        strides_count: u16,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Store an element through element.addr + store.
    ElementStore {
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Allocate managed memory.
    ManagedAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        layout: mir::LocalNodeId<mir::Type>,
        layout_id: Option<LayoutId>,
        byte_len: u32,
        trace: ReferenceMap,
    },

    /// Allocate managed array.
    ManagedAllocArray {
        dest: mir::Value,
        length: mir::Value,
        reference: ReferenceMeta,
        element_type: mir::LocalNodeId<mir::Type>,
    },

    /// Allocate raw memory.
    RawAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        byte_len: u32,
    },

    /// Free raw memory (user-inserted, FFI).
    RawFree { pointer: mir::Value },

    /// Drop raw memory (compiler-inserted, ownership end).
    RawDrop { value: mir::Value },

    /// Allocate stack memory.
    StackAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        slot_count: u32,
    },

    /// Mark stack value lifetime ended (compiler-inserted, NLL).
    StackDrop { value: mir::Value },

    /// Assume a condition is true (UB if false).
    Assume { condition: mir::Value },

    /// Intrinsic call.
    Intrinsic {
        dest: mir::Value,
        intrinsic: mir::Intrinsic,
        arguments: ArgumentRange,
    },

    /// Atomic load.
    AtomicLoad {
        dest: mir::Value,
        pointer: mir::Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Atomic store.
    AtomicStore {
        pointer: mir::Value,
        value: mir::Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Atomic compare exchange.
    AtomicCompareExchange {
        dest: mir::Value,
        pointer: mir::Value,
        expected: mir::Value,
        new_value: mir::Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Atomic read modify write.
    AtomicRmw {
        dest: mir::Value,
        operator: mir::AtomicRmwOperator,
        pointer: mir::Value,
        value: mir::Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Atomic fence.
    AtomicFence,

    /// Synchronization barrier.
    Barrier,

    /// Return from function.
    Return { value: mir::Value },

    /// Yield from a coroutine.
    Yield {
        /// The value to yield.
        value: mir::Value,
        /// Resume block index.
        resume_block: u32,
        /// Copy plan for resume arguments.
        resume_copies: CopyRange,
        /// Destination for the resumed value.
        resume_value: mir::Value,
    },

    /// Unconditional jump.
    Jump { target: u32, copies: CopyRange },

    /// Conditional branch.
    Branch {
        condition: mir::Value,
        then_target: u32,
        then_copies: CopyRange,
        else_target: u32,
        else_copies: CopyRange,
    },

    /// Fused compare and branch (icmp + branch).
    CompareAndBranch {
        left: mir::Value,
        right: mir::Value,
        operator: mir::BinaryOperator,
        then_target: u32,
        then_copies: CopyRange,
        else_target: u32,
        else_copies: CopyRange,
    },

    /// Fused compare and branch with constant right operand.
    CompareAndBranchConst {
        left: mir::Value,
        right_const: Value,
        operator: mir::BinaryOperator,
        then_target: u32,
        then_copies: CopyRange,
        else_target: u32,
        else_copies: CopyRange,
    },

    /// Switch on integer.
    Switch {
        value: mir::Value,
        cases: SwitchRange,
        default_target: u32,
        default_copies: CopyRange,
    },

    /// Switch via dense jump table.
    SwitchTable {
        value: mir::Value,
        min: i64,
        table: SwitchRange,
        default_target: u32,
        default_copies: CopyRange,
    },

    /// Unrecoverable runtime termination.
    Trap {
        kind: mir::TrapKind,
        payload: mir::Value,
    },

    /// Unreachable code.
    Unreachable,

    /// Tail call to a function (call + return).
    TailCall {
        function: u32,
        callee_index: u32,
        copies: CopyRange,
    },

    /// Tail call to the current function (fast path).
    TailCallSelf {
        /// Entry block index for the current function.
        entry: u32,
        /// Arguments to pass.
        arguments: ArgumentRange,
    },

    /// Virtual tail call (call + return).
    TailCallVirtual {
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface tail call (call + return).
    TailCallInterface {
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Indirect tail call (call + return).
    TailCallIndirect {
        callee: mir::Value,
        env: Option<mir::Value>,
        arguments: ArgumentRange,
        cached_function: Cell<Option<u32>>,
        cached_ptr: Cell<Option<NonNull<ThreadedFunction>>>,
    },
}

impl ThreadedInstructionData {
    /// Return a short opcode label for instruction profiling.
    #[cfg(feature = "stats")]
    pub fn opcode_name(&self) -> &'static str {
        match self {
            ThreadedInstructionData::Const { .. } => "const",
            ThreadedInstructionData::Binary { .. } => "binary",
            ThreadedInstructionData::BinaryElementwise { .. } => "binary_elementwise",
            ThreadedInstructionData::BinarySpecialized { .. } => "binary_specialized",
            ThreadedInstructionData::BinaryConstRight { .. } => "binary_const_right",
            ThreadedInstructionData::BinaryConstRightSpecialized { .. } => {
                "binary_const_right_specialized"
            }
            ThreadedInstructionData::Unary { .. } => "unary",
            ThreadedInstructionData::UnaryElementwise { .. } => "unary_elementwise",
            ThreadedInstructionData::Cast { .. } => "cast",
            ThreadedInstructionData::Select { .. } => "select",
            ThreadedInstructionData::Call { .. } => "call",
            ThreadedInstructionData::CallVirtual { .. } => "call_virtual",
            ThreadedInstructionData::CallInterface { .. } => "call_interface",
            ThreadedInstructionData::CallIndirect { .. } => "call_indirect",
            ThreadedInstructionData::LocalGet { .. } => "local_get",
            ThreadedInstructionData::LocalAddr { .. } => "local_addr",
            ThreadedInstructionData::LocalSet { .. } => "local_set",
            ThreadedInstructionData::GlobalAddr { .. } => "global_addr",
            ThreadedInstructionData::GlobalConst { .. } => "global_const",
            ThreadedInstructionData::FunctionAddr { .. } => "function_addr",
            ThreadedInstructionData::FunctionEnv { .. } => "function_env",
            ThreadedInstructionData::GlobalLoad { .. } => "global_load",
            ThreadedInstructionData::GlobalStore { .. } => "global_store",
            ThreadedInstructionData::Load { .. } => "load",
            ThreadedInstructionData::Store { .. } => "store",
            ThreadedInstructionData::FieldGet { .. } => "field_get",
            ThreadedInstructionData::FieldAddr { .. } => "field_addr",
            ThreadedInstructionData::FieldLoad { .. } => "field_load",
            ThreadedInstructionData::FieldSet { .. } => "field_set",
            ThreadedInstructionData::FieldStore { .. } => "field_store",
            ThreadedInstructionData::ElementGet { .. } => "element_get",
            ThreadedInstructionData::ElementAddr { .. } => "element_addr",
            ThreadedInstructionData::ElementLoad { .. } => "element_load",
            ThreadedInstructionData::ElementSet { .. } => "element_set",
            ThreadedInstructionData::ElementStore { .. } => "element_store",
            ThreadedInstructionData::Aggregate { .. } => "aggregate",
            ThreadedInstructionData::VectorSplat { .. } => "vector_splat",
            ThreadedInstructionData::VectorExtract { .. } => "vector_extract",
            ThreadedInstructionData::VectorInsert { .. } => "vector_insert",
            ThreadedInstructionData::VectorShuffle { .. } => "vector_shuffle",
            ThreadedInstructionData::VectorSelect { .. } => "vector_select",
            ThreadedInstructionData::VectorReduce { .. } => "vector_reduce",
            ThreadedInstructionData::VectorCompare { .. } => "vector_compare",
            ThreadedInstructionData::VectorConvert { .. } => "vector_convert",
            ThreadedInstructionData::TensorLoad { .. } => "tensor_load",
            ThreadedInstructionData::TensorStore { .. } => "tensor_store",
            ThreadedInstructionData::TensorFill { .. } => "tensor_fill",
            ThreadedInstructionData::TensorCopy { .. } => "tensor_copy",
            ThreadedInstructionData::TensorReshape { .. } => "tensor_reshape",
            ThreadedInstructionData::TensorBroadcast { .. } => "tensor_broadcast",
            ThreadedInstructionData::TensorTranspose { .. } => "tensor_transpose",
            ThreadedInstructionData::TensorCast { .. } => "tensor_cast",
            ThreadedInstructionData::TensorView { .. } => "tensor_view",
            ThreadedInstructionData::TensorSlice { .. } => "tensor_slice",
            ThreadedInstructionData::TensorPad { .. } => "tensor_pad",
            ThreadedInstructionData::TensorConcat { .. } => "tensor_concat",
            ThreadedInstructionData::TensorReduce { .. } => "tensor_reduce",
            ThreadedInstructionData::TensorDot { .. } => "tensor_dot",
            ThreadedInstructionData::TensorConvolution { .. } => "tensor_convolution",
            ThreadedInstructionData::TensorGather { .. } => "tensor_gather",
            ThreadedInstructionData::TensorScatter { .. } => "tensor_scatter",
            ThreadedInstructionData::TensorCompare { .. } => "tensor_compare",
            ThreadedInstructionData::TensorSelect { .. } => "tensor_select",
            ThreadedInstructionData::TensorConvert { .. } => "tensor_convert",
            ThreadedInstructionData::ManagedAlloc { .. } => "managed_alloc",
            ThreadedInstructionData::ManagedAllocArray { .. } => "managed_alloc_array",
            ThreadedInstructionData::RawAlloc { .. } => "raw_alloc",
            ThreadedInstructionData::RawFree { .. } => "raw_free",
            ThreadedInstructionData::RawDrop { .. } => "raw_drop",
            ThreadedInstructionData::StackAlloc { .. } => "stack_alloc",
            ThreadedInstructionData::StackDrop { .. } => "stack_drop",
            ThreadedInstructionData::Assume { .. } => "assume",
            ThreadedInstructionData::Intrinsic { .. } => "intrinsic",
            ThreadedInstructionData::AtomicLoad { .. } => "atomic_load",
            ThreadedInstructionData::AtomicStore { .. } => "atomic_store",
            ThreadedInstructionData::AtomicCompareExchange { .. } => "atomic_compare_exchange",
            ThreadedInstructionData::AtomicRmw { .. } => "atomic_rmw",
            ThreadedInstructionData::AtomicFence => "atomic_fence",
            ThreadedInstructionData::Barrier => "barrier",
            ThreadedInstructionData::Return { .. } => "return",
            ThreadedInstructionData::Yield { .. } => "yield",
            ThreadedInstructionData::Jump { .. } => "jump",
            ThreadedInstructionData::Branch { .. } => "branch",
            ThreadedInstructionData::CompareAndBranch { .. } => "compare_and_branch",
            ThreadedInstructionData::CompareAndBranchConst { .. } => "compare_and_branch_const",
            ThreadedInstructionData::Switch { .. } => "switch",
            ThreadedInstructionData::SwitchTable { .. } => "switch_table",
            ThreadedInstructionData::Trap { .. } => "trap",
            ThreadedInstructionData::Unreachable => "unreachable",
            ThreadedInstructionData::TailCall { .. } => "tail_call",
            ThreadedInstructionData::TailCallSelf { .. } => "tail_call_self",
            ThreadedInstructionData::TailCallVirtual { .. } => "tail_call_virtual",
            ThreadedInstructionData::TailCallInterface { .. } => "tail_call_interface",
            ThreadedInstructionData::TailCallIndirect { .. } => "tail_call_indirect",
        }
    }
}

/// Switch case.
#[derive(Clone, Debug)]
pub struct SwitchCase {
    /// Match value.
    pub value: i64,
    /// Target block.
    pub target: u32,
    /// Block parameter copies.
    pub copies: CopyRange,
}

/// Threaded basic block.
#[derive(Clone, Debug)]
pub struct ThreadedBlock {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Block parameters.
    pub parameters: ArgumentRange,
    /// Instructions including terminator.
    pub instructions: Vec<ThreadedInstruction>,
    /// Original MIR instruction count (before fusion/threading).
    pub mir_instruction_count: u32,
}

/// Threaded function with optimized dispatch.
#[derive(Clone, Debug)]
pub struct ThreadedFunction {
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// All blocks.
    pub blocks: Vec<ThreadedBlock>,
    /// Pool of argument values referenced by ranges.
    pub argument_pool: Vec<mir::Value>,
    /// Pool of switch cases referenced by ranges.
    pub switch_case_pool: Vec<SwitchCase>,
    /// Pool of value copy pairs referenced by ranges.
    pub copy_pool: Vec<CopyPair>,
    /// Count of SSA values used by the function.
    pub value_count: usize,
    /// Count of local variables used by the function.
    pub local_count: usize,
}

/// Copy pair for parameter binding.
#[derive(Clone, Copy, Debug)]
pub struct CopyPair {
    /// Destination SSA value id.
    pub dest: u32,
    /// Source SSA value id (sentinel for missing).
    pub src: u32,
}

/// Execution state for threaded interpreter.
pub struct ThreadedState<'ctx, 'iso> {
    /// Index of the current frame in the call stack.
    pub frame_index: usize,
    /// Interpreter reference for heap and globals.
    pub(crate) interpreter: &'ctx mut InterpreterContext<'iso>,
    /// Whether bounds checks are enabled for this execution.
    pub bounds_checks: bool,
    /// Whether null checks are enabled for this execution.
    pub null_checks: bool,
    /// Whether to collect execution statistics.
    pub collect_stats: bool,
    /// Pointer to the current frame for fast access.
    frame: *mut Frame,
    /// Pointer to SSA value storage for this frame.
    values: *mut Value,
    /// Count of SSA values in this frame.
    value_count: usize,
    /// Pointer to local variable storage for this frame.
    locals: *mut Value,
    /// Count of local variables in this frame.
    local_count: usize,
    /// Argument pool for the current function.
    argument_pool: *const mir::Value,
    /// Argument pool length.
    argument_pool_len: usize,
    /// Switch case pool for the current function.
    switch_case_pool: *const SwitchCase,
    /// Switch case pool length.
    switch_case_pool_len: usize,
}

impl fmt::Debug for ThreadedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadedInstruction")
            .field("data", &self.data)
            .finish()
    }
}

impl fmt::Debug for ThreadedState<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadedState")
            .field("frame_index", &self.frame_index)
            .field("value_count", &self.value_count)
            .field("local_count", &self.local_count)
            .field("argument_pool_len", &self.argument_pool_len)
            .field("switch_case_pool_len", &self.switch_case_pool_len)
            .field("bounds_checks", &self.bounds_checks)
            .field("null_checks", &self.null_checks)
            .field("collect_stats", &self.collect_stats)
            .finish()
    }
}

impl<'ctx, 'iso> ThreadedState<'ctx, 'iso> {
    /// Create state for the current frame.
    pub(crate) fn new(
        interpreter: &'ctx mut InterpreterContext<'iso>,
        frame_index: usize,
        argument_pool: &[mir::Value],
        switch_case_pool: &[SwitchCase],
    ) -> Self {
        // resolve check policies
        let mode = interpreter.isolate.image.options.execution.mode;
        let bounds_checks = interpreter
            .isolate
            .image
            .options
            .checks
            .bounds
            .is_enabled_for(mode);
        let null_checks = interpreter
            .isolate
            .image
            .options
            .checks
            .null
            .is_enabled_for(mode);
        let collect_stats = interpreter.isolate.image.options.telemetry.collect_stats;

        // get frame pointer
        // #Safety: frame_index always points at the current frame
        let frame =
            unsafe { interpreter.engine.call_stack.get_unchecked_mut(frame_index) as *mut Frame };

        // load frame bounds
        let value_base = unsafe { (*frame).value_base };
        let value_count = unsafe { (*frame).value_count };
        let local_base = unsafe { (*frame).local_base };
        let local_count = unsafe { (*frame).local_count };

        // validate stack bounds in debug builds
        debug_assert!(
            value_base + value_count <= interpreter.engine.value_stack.len(),
            "value stack out of bounds for frame"
        );
        debug_assert!(
            local_base + local_count <= interpreter.engine.local_stack.len(),
            "local stack out of bounds for frame"
        );

        // cache stack pointers
        let values_ptr = interpreter.engine.value_stack.as_mut_ptr();
        let locals_ptr = interpreter.engine.local_stack.as_mut_ptr();

        // assemble state
        Self {
            frame_index,
            interpreter,
            bounds_checks,
            null_checks,
            collect_stats,
            frame,
            values: unsafe { values_ptr.add(value_base) },
            value_count,
            locals: unsafe { locals_ptr.add(local_base) },
            local_count,
            argument_pool: argument_pool.as_ptr(),
            argument_pool_len: argument_pool.len(),
            switch_case_pool: switch_case_pool.as_ptr(),
            switch_case_pool_len: switch_case_pool.len(),
        }
    }

    /// Refresh cached pointers for the current frame and threaded function.
    pub fn refresh_for_threaded(&mut self, threaded: &ThreadedFunction) {
        // load frame bounds
        let (value_base, value_count, local_base, local_count) = {
            let frame = self.current_frame_mut();
            (
                frame.value_base,
                frame.value_count,
                frame.local_base,
                frame.local_count,
            )
        };

        // validate stack bounds in debug builds
        debug_assert!(
            value_base + value_count <= self.interpreter.engine.value_stack.len(),
            "value stack out of bounds for frame"
        );
        debug_assert!(
            local_base + local_count <= self.interpreter.engine.local_stack.len(),
            "local stack out of bounds for frame"
        );

        // cache stack pointers
        let values_ptr = self.interpreter.engine.value_stack.as_mut_ptr();
        let locals_ptr = self.interpreter.engine.local_stack.as_mut_ptr();
        self.values = unsafe { values_ptr.add(value_base) };
        self.locals = unsafe { locals_ptr.add(local_base) };
        self.value_count = value_count;
        self.local_count = local_count;

        // refresh argument and switch pools
        self.argument_pool = threaded.argument_pool.as_ptr();
        self.argument_pool_len = threaded.argument_pool.len();
        self.switch_case_pool = threaded.switch_case_pool.as_ptr();
        self.switch_case_pool_len = threaded.switch_case_pool.len();
    }

    /// Borrow the heap for the current block.
    #[inline]
    pub(crate) fn heap(&mut self) -> &mut Heap {
        self.interpreter.heap()
    }

    /// Borrow the heap immutably for the current block.
    #[inline]
    pub(crate) fn heap_ref(&self) -> &Heap {
        self.interpreter.heap_ref()
    }

    /// Execute one intrinsic against the current interpreter and heap state.
    pub(crate) fn execute_intrinsic(
        &mut self,
        intrinsic: mir::Intrinsic,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.interpreter.execute_intrinsic_resolved(intrinsic, args)
    }

    /// Allocate an aggregate on the managed heap.
    #[inline]
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        let handle = self
            .heap()
            .allocate_packed_values(values)
            .unwrap_or_else(|error| panic!("{error}"));
        Value::aggregate(handle)
    }

    /// Allocate a 2-element aggregate on the managed heap.
    #[inline]
    pub(crate) fn allocate_pair(&mut self, first: Value, second: Value) -> Value {
        let handle = self
            .heap()
            .allocate_packed_pair(first, second)
            .unwrap_or_else(|error| panic!("{error}"));
        Value::aggregate(handle)
    }

    /// Allocate a 1-element aggregate on the managed heap.
    #[inline]
    pub(crate) fn allocate_single(&mut self, value: Value) -> Value {
        let handle = self
            .heap()
            .allocate_packed_single(value)
            .unwrap_or_else(|error| panic!("{error}"));
        Value::aggregate(handle)
    }

    /// Move the state to a new frame and threaded function.
    pub fn enter_frame(&mut self, frame_index: usize, threaded: &ThreadedFunction) {
        // validate frame index in debug builds
        debug_assert!(
            frame_index < self.interpreter.engine.call_stack.len(),
            "frame index out of bounds"
        );

        // update cached frame pointer
        let frame = unsafe {
            self.interpreter
                .engine
                .call_stack
                .get_unchecked_mut(frame_index)
        };
        self.frame_index = frame_index;
        self.frame = frame as *mut Frame;

        // refresh cached pointers
        self.refresh_for_threaded(threaded);
    }

    /// Record a profile sample for the current instruction when enabled.
    #[inline(always)]
    pub fn maybe_profile_instruction(&mut self, instruction: &ThreadedInstruction) {
        #[cfg(feature = "stats")]
        if let Some(profile) = self.interpreter.engine.instruction_profile.as_mut() {
            profile.maybe_sample(instruction.data.opcode_name());
        }
        #[cfg(not(feature = "stats"))]
        {
            let _ = instruction;
        }
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub fn current_frame_mut(&mut self) -> &mut Frame {
        // #Safety: frame pointer is valid for current block execution
        unsafe { &mut *self.frame }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub fn frame_by_index(&self, frame_index: usize) -> Result<&Frame, Error> {
        self.interpreter
            .engine
            .call_stack
            .get(frame_index)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Get a frame by index mutably.
    #[inline(always)]
    pub fn frame_by_index_mut(&mut self, frame_index: usize) -> Result<&mut Frame, Error> {
        self.interpreter
            .engine
            .call_stack
            .get_mut(frame_index)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub fn get(&self, v: mir::Value) -> Value {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe { *self.values.add(index) }
    }

    /// Set value by SSA id.
    #[inline(always)]
    pub fn set(&mut self, v: mir::Value, val: Value) {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe {
            *self.values.add(index) = val;
        }
    }

    /// Get local variable by local index.
    #[inline(always)]
    pub fn get_local_by_index(&self, local_index: u32) -> Value {
        let index = local_index as usize;
        debug_assert!(
            index < self.local_count,
            "local out of bounds: {local_index}"
        );
        unsafe { *self.locals.add(index) }
    }

    /// Set local variable by local index.
    #[inline(always)]
    pub fn set_local_by_index(&mut self, local_index: u32, val: Value) {
        let index = local_index as usize;
        debug_assert!(
            index < self.local_count,
            "local out of bounds: {local_index}"
        );
        unsafe {
            *self.locals.add(index) = val;
        }
    }

    /// Get the argument slice for the given range.
    #[inline(always)]
    pub fn argument_slice(&self, range: ArgumentRange) -> &[mir::Value] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.argument_pool_len,
            "argument pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.argument_pool.add(start), len) }
    }

    /// Get the switch case slice for the given range.
    #[inline(always)]
    pub fn switch_cases(&self, range: SwitchRange) -> &[SwitchCase] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.switch_case_pool_len,
            "switch case pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.switch_case_pool.add(start), len) }
    }
}
