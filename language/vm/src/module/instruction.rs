use std::fmt;

use destack_mir::LayoutId;
use {destack_engine as engine, destack_mir as mir};

use crate::{ReferenceMeta, Value};

use super::{ArgumentRange, CallTarget, CopyRange, Opcode, SwitchRange};

/// One decoded module instruction.
#[derive(Clone)]
pub(crate) struct Instruction {
    /// The instruction opcode.
    pub opcode: Opcode,
    /// The encoded immediate for this opcode.
    pub immediate: Immediate,
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction")
            .field("immediate", &self.immediate)
            .finish()
    }
}

/// Constant payload stored in lowered instructions.
#[derive(Clone, Debug)]
pub(crate) enum ConstValue {
    /// Pre-decoded constant value.
    Value(Value),
}

/// One compiled field access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAccess {
    /// The field value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field payload.
    pub byte_offset: usize,
    /// The byte width of the field payload.
    pub byte_len: usize,
    /// Whether the field payload decodes as one scalar.
    pub is_scalar: bool,
}

/// One compiled element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementAccess {
    /// The element value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The byte stride between adjacent elements.
    pub byte_stride: usize,
    /// The byte width of the element payload.
    pub byte_len: usize,
    /// Whether the element payload decodes as one scalar.
    pub is_scalar: bool,
}

/// One compiled pointee access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TypedAccess {
    /// The pointee value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The byte width of the pointee payload.
    pub byte_len: usize,
    /// Whether the pointee payload decodes as one scalar.
    pub is_scalar: bool,
}

impl From<FieldAccess> for TypedAccess {
    fn from(field: FieldAccess) -> Self {
        Self {
            value_type: field.value_type,
            byte_len: field.byte_len,
            is_scalar: field.is_scalar,
        }
    }
}

impl From<ElementAccess> for TypedAccess {
    fn from(element: ElementAccess) -> Self {
        Self {
            value_type: element.value_type,
            byte_len: element.byte_len,
            is_scalar: element.is_scalar,
        }
    }
}

/// Instruction immediate.
#[derive(Clone, Debug)]
pub(crate) enum Immediate {
    /// Load constant.
    Const { dest: mir::Value, value: ConstValue },

    /// Binary opcode.
    Binary {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    },

    /// Elementwise binary opcode on vector or tensor values.
    BinaryElementwise {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        result_type: mir::LocalNodeId<mir::Type>,
    },

    /// Specialized binary opcode.
    BinarySpecialized {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
    },

    /// Binary opcode with constant right operand.
    BinaryConstRight {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right_const: Value,
    },

    /// Specialized binary with constant right.
    BinaryConstRightSpecialized {
        dest: mir::Value,
        left: mir::Value,
        right_const: Value,
    },

    /// Unary opcode.
    Unary {
        dest: mir::Value,
        op: mir::UnaryOperator,
        arg: mir::Value,
    },

    /// Elementwise unary opcode on vector or tensor values.
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
        target: CallTarget,
        arguments: ArgumentRange,
        copies: CopyRange,
    },

    /// Function call terminator with explicit normal and unwind continuations.
    CallBranch {
        function: u32,
        target: CallTarget,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
    },

    /// Virtual method call.
    CallVirtual {
        dest: mir::Value,
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Virtual method call terminator with explicit normal and unwind continuations.
    CallVirtualBranch {
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
    },

    /// Interface method call.
    CallInterface {
        dest: mir::Value,
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface method call terminator with explicit normal and unwind continuations.
    CallInterfaceBranch {
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
    },

    /// Indirect function call.
    CallIndirect {
        dest: mir::Value,
        callee: mir::Value,
        arguments: ArgumentRange,
    },

    /// Indirect call terminator with explicit normal and unwind continuations.
    CallIndirectBranch {
        callee: mir::Value,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
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

    /// Get a function pointer.
    FunctionAddr { dest: mir::Value, function: u32 },

    /// Bind one environment to a function value.
    CallableBind {
        dest: mir::Value,
        function: u32,
        environment: mir::Value,
    },

    /// Load the callable environment pointer.
    CallableEnvironment { dest: mir::Value },

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
        access: Option<TypedAccess>,
    },

    /// Store to pointer.
    Store {
        pointer: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        access: Option<TypedAccess>,
    },

    /// Get struct or tuple field.
    FieldGet {
        dest: mir::Value,
        composite: mir::Value,
        index: u32,
        field_count: Option<u32>,
        field: Option<FieldAccess>,
    },

    /// Get struct or tuple field address.
    FieldAddr {
        dest: mir::Value,
        composite: mir::Value,
        index: u32,
        reference: ReferenceMeta,
        field_count: Option<u32>,
        field: Option<FieldAccess>,
    },

    /// Load a field through field address plus load.
    FieldLoad {
        dest: mir::Value,
        composite: mir::Value,
        index: u32,
        field_count: Option<u32>,
        field: Option<FieldAccess>,
    },

    /// Set struct or tuple field.
    FieldSet {
        dest: mir::Value,
        composite: mir::Value,
        index: u32,
        value: mir::Value,
        field_count: Option<u32>,
        field: Option<FieldAccess>,
    },

    /// Store a field through field address plus store.
    FieldStore {
        composite: mir::Value,
        index: u32,
        value: mir::Value,
        reference: ReferenceMeta,
        field_count: Option<u32>,
        field: Option<FieldAccess>,
    },

    /// Get array element.
    ElementGet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        array_length: Option<u64>,
        element: Option<ElementAccess>,
    },

    /// Get array element address.
    ElementAddr {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        array_length: Option<u64>,
        element: Option<ElementAccess>,
    },

    /// Load an element through element address plus load.
    ElementLoad {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        array_length: Option<u64>,
        element: Option<ElementAccess>,
    },

    /// Set array element.
    ElementSet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
        array_length: Option<u64>,
        element: Option<ElementAccess>,
    },

    /// Construct a composite from element values.
    Composite {
        dest: mir::Value,
        elements: ArgumentRange,
    },

    /// Broadcast a scalar to all vector lanes.
    VectorSplat { dest: mir::Value, value: mir::Value },

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
        element: Option<ElementAccess>,
    },

    /// Broadcast a scalar to all tensor elements.
    TensorSplat {
        dest: mir::Value,
        value: mir::Value,
        tensor_type: mir::LocalNodeId<mir::Type>,
    },

    /// Extract a tensor element from a tensor value.
    TensorExtract {
        dest: mir::Value,
        tensor: mir::Value,
        indices: ArgumentRange,
        tensor_type: mir::LocalNodeId<mir::Type>,
    },

    /// Store a tensor element into a view.
    TensorStore {
        view: mir::Value,
        indices: ArgumentRange,
        value: mir::Value,
        view_type: mir::LocalNodeId<mir::Type>,
        element: Option<ElementAccess>,
    },

    /// Fill a tensor reference with a scalar value.
    TensorFill {
        view: mir::Value,
        value: mir::Value,
        view_type: mir::LocalNodeId<mir::Type>,
        element: Option<ElementAccess>,
    },

    /// Copy elements between tensor references.
    TensorCopy {
        target: mir::Value,
        source: mir::Value,
        target_type: mir::LocalNodeId<mir::Type>,
        source_type: mir::LocalNodeId<mir::Type>,
        target_element: Option<ElementAccess>,
        source_element: Option<ElementAccess>,
    },

    /// Reshape a tensor into a new shape.
    TensorReshape {
        dest: mir::Value,
        tensor: mir::Value,
        shape: ArgumentRange,
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
        dest: mir::Value,
        tensors: ArgumentRange,
        tensor_types: Vec<mir::LocalNodeId<mir::Type>>,
        axis: u32,
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
        element: Option<ElementAccess>,
    },

    /// Store an element through element address plus store.
    ElementStore {
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        array_length: Option<u64>,
        element: Option<ElementAccess>,
    },

    /// Allocate heap storage.
    New {
        dest: mir::Value,
        reference: ReferenceMeta,
        storage_type: mir::LocalNodeId<mir::Type>,
        layout_id: Option<LayoutId>,
    },

    /// Allocate a heap array.
    NewSlice {
        dest: mir::Value,
        length: mir::Value,
        slice_type: mir::LocalNodeId<mir::Type>,
    },

    /// Allocate raw memory.
    RawAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        byte_len: usize,
    },

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Run explicit synchronous cleanup.
    Dispose { value: mir::Value },

    /// Run explicit asynchronous cleanup.
    AsyncDispose { value: mir::Value },

    /// Pin one local heap reference.
    Pin { value: mir::Value },

    /// Release one local heap pin.
    Unpin { value: mir::Value },

    /// End ownership synchronously.
    Drop { value: mir::Value },

    /// Allocate stack memory.
    StackAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        storage_type: mir::LocalNodeId<mir::Type>,
    },

    /// Assume a condition is true.
    Assume,

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
        value: mir::Value,
        source: mir::Value,
        resume_point: engine::ResumePointId,
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

    /// Semantic check with explicit success and failure edges.
    Check {
        constraint: mir::CheckConstraint,
        then_target: u32,
        then_copies: CopyRange,
        else_target: u32,
        else_copies: CopyRange,
    },

    /// Fused compare and branch.
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

    /// Throw one managed exception object.
    Throw { value: mir::Value },

    /// Unreachable code.
    Unreachable,

    /// Tail call to a function.
    TailCall {
        function: u32,
        target: CallTarget,
        copies: CopyRange,
    },

    /// Tail call to the current function.
    TailCallSelf {
        entry: u32,
        arguments: ArgumentRange,
    },

    /// Virtual tail call.
    TailCallVirtual {
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface tail call.
    TailCallInterface {
        receiver: mir::Value,
        heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Indirect tail call.
    TailCallIndirect {
        callee: mir::Value,
        arguments: ArgumentRange,
    },
}
