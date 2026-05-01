use {destack_engine as engine, destack_mir as mir};

use crate::ReferenceMeta;

use super::{
    AllocationLayoutId, ArgumentRange, CallTarget, CheckId, ConstValue, ElementAccessId,
    FieldAccessId, FrameAccessId, MoveRange, PointeeAccessId, PointerClass, SliceElementAccessId,
    SwitchCasesId, SwitchTableId, TensorConvolutionId, TensorDotId, TensorGatherId,
    TensorScatterId, TensorWindowId, TypeRangeId, U32RangeId,
};

/// Instruction operands.
#[derive(Clone, Debug)]
pub(crate) enum Operands {
    /// Load one constant value.
    LoadConst { dest: mir::Value, value: ConstValue },

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

    /// Integer binary opcode.
    BinaryInteger {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
        width: u8,
        is_signed: bool,
    },

    /// Unary opcode.
    Unary {
        dest: mir::Value,
        op: mir::UnaryOperator,
        arg: mir::Value,
    },

    /// Integer unary opcode.
    UnaryInteger {
        dest: mir::Value,
        arg: mir::Value,
        width: u8,
        is_signed: bool,
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
        moves: MoveRange,
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
        table_field: Option<FieldAccessId>,
        method_index: u32,
        arguments: ArgumentRange,
    },

    /// Virtual method call terminator with explicit normal and unwind continuations.
    CallVirtualBranch {
        receiver: mir::Value,
        table_field: Option<FieldAccessId>,
        method_index: u32,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
    },

    /// Interface method call.
    CallInterface {
        dest: mir::Value,
        receiver: mir::Value,
        table_field: Option<FieldAccessId>,
        method_index: u32,
        arguments: ArgumentRange,
    },

    /// Interface method call terminator with explicit normal and unwind continuations.
    CallInterfaceBranch {
        receiver: mir::Value,
        table_field: Option<FieldAccessId>,
        method_index: u32,
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

    /// Get static address.
    StaticAddr {
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

    /// Fused static address and load.
    StaticLoad { dest: mir::Value, global: u32 },

    /// Fused static address and store.
    StaticStore {
        global: u32,
        value: mir::Value,
        reference: ReferenceMeta,
    },

    /// Load from pointer.
    Load {
        dest: mir::Value,
        pointer: mir::Value,
        access: PointeeAccessId,
    },

    /// Load one word through a frame value access.
    LoadFrame {
        dest: mir::Value,
        base: mir::Value,
        index: mir::Value,
        access: FrameAccessId,
    },

    /// Store to pointer.
    Store {
        pointer: mir::Value,
        value: mir::Value,
        access: PointeeAccessId,
    },

    /// Store one word through a frame value access.
    StoreFrame {
        base: mir::Value,
        index: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        access: FrameAccessId,
    },

    /// Compute a frame value address.
    AddressFrame {
        dest: mir::Value,
        base: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        access: FrameAccessId,
    },

    /// Copy bytes between frame values.
    CopyFrame {
        destination: mir::Value,
        destination_index: mir::Value,
        destination_access: FrameAccessId,
        source: mir::Value,
        source_index: mir::Value,
        source_access: FrameAccessId,
    },

    /// Load bytes from an address into a frame value.
    LoadFrameBytes {
        destination: mir::Value,
        address: mir::Value,
        access: PointeeAccessId,
    },

    /// Store bytes from a frame value into an address.
    StoreFrameBytes {
        address: mir::Value,
        source: mir::Value,
        access: PointeeAccessId,
    },

    /// Get struct or tuple field address.
    FieldAddr {
        dest: mir::Value,
        base: mir::Value,
        index: u32,
        reference: ReferenceMeta,
        field_count: u32,
        field: FieldAccessId,
    },

    /// Load a field through field address plus load.
    FieldLoad {
        dest: mir::Value,
        base: mir::Value,
        index: u32,
        field_count: u32,
        field: FieldAccessId,
    },

    /// Store a field through field address plus store.
    FieldStore {
        base: mir::Value,
        index: u32,
        value: mir::Value,
        reference: ReferenceMeta,
        field_count: u32,
        field: FieldAccessId,
    },

    /// Get array element address.
    ElementAddr {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
        element: ElementAccessId,
    },

    /// Get slice element address.
    SliceElementAddr {
        dest: mir::Value,
        slice: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        access: SliceElementAccessId,
    },

    /// Load an element through element address plus load.
    ElementLoad {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        array_length: u64,
        element: ElementAccessId,
    },

    /// Broadcast a scalar to all vector elements.
    VectorSplat { dest: mir::Value, value: mir::Value },

    /// Extract an element from a vector.
    VectorExtract {
        dest: mir::Value,
        vector: mir::Value,
        index: mir::Value,
    },

    /// Insert an element into a vector.
    VectorInsert {
        dest: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        value: mir::Value,
    },

    /// Shuffle vector elements using a constant mask.
    VectorShuffle {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
        mask: U32RangeId,
    },

    /// Select vector elements based on a boolean mask.
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
        element: Option<ElementAccessId>,
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
        element: Option<ElementAccessId>,
    },

    /// Fill a tensor reference with a scalar value.
    TensorFill {
        view: mir::Value,
        value: mir::Value,
        view_type: mir::LocalNodeId<mir::Type>,
        element: Option<ElementAccessId>,
    },

    /// Copy elements between tensor references.
    TensorCopy {
        target: mir::Value,
        source: mir::Value,
        target_type: mir::LocalNodeId<mir::Type>,
        source_type: mir::LocalNodeId<mir::Type>,
        target_element: Option<ElementAccessId>,
        source_element: Option<ElementAccessId>,
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
        dimensions: U32RangeId,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Permute tensor dimensions.
    TensorTranspose {
        dest: mir::Value,
        tensor: mir::Value,
        permutation: U32RangeId,
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
        tensor_types: TypeRangeId,
        axis: u32,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Reduce a tensor along axes.
    TensorReduce {
        dest: mir::Value,
        operator: mir::TensorReduceOperator,
        tensor: mir::Value,
        initial: mir::Value,
        axes: U32RangeId,
        source_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Dot product of two tensors.
    TensorDot {
        dest: mir::Value,
        left: mir::Value,
        right: mir::Value,
        dimensions: TensorDotId,
        left_type: mir::LocalNodeId<mir::Type>,
        right_type: mir::LocalNodeId<mir::Type>,
        dest_type: mir::LocalNodeId<mir::Type>,
    },

    /// Convolution between an input tensor and a kernel tensor.
    TensorConvolution {
        dest: mir::Value,
        input: mir::Value,
        kernel: mir::Value,
        dimensions: TensorConvolutionId,
        window: TensorWindowId,
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
        dimensions: TensorGatherId,
        slice_sizes: U32RangeId,
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
        dimensions: TensorScatterId,
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
        element: Option<ElementAccessId>,
    },

    /// Store an element through element address plus store.
    ElementStore {
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
        element: ElementAccessId,
    },

    /// Allocate a managed heap value.
    New {
        dest: mir::Value,
        allocation: AllocationLayoutId,
    },

    /// Allocate a heap array.
    NewSlice {
        dest: mir::Value,
        length: mir::Value,
        pointer_class: PointerClass,
        element: AllocationLayoutId,
        element_alignment: usize,
    },

    /// Allocate raw memory.
    RawAlloc { dest: mir::Value, byte_len: usize },

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Run explicit synchronous cleanup.
    Dispose,

    /// Run explicit asynchronous cleanup.
    AsyncDispose,

    /// Pin one local heap reference.
    Pin { value: mir::Value },

    /// Release one local heap pin.
    Unpin { value: mir::Value },

    /// End ownership synchronously.
    Drop { value: mir::Value },

    /// Allocate stack bytes.
    StackAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        allocation_type: mir::LocalNodeId<mir::Type>,
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

    /// Managed barrier write.
    BarrierWrite {
        /// The managed object whose reference range changed.
        object: mir::Value,
        /// The byte offset of the changed reference range.
        offset: mir::Value,
        /// The changed byte length.
        byte_len: mir::Value,
        /// The managed heap class for the object reference.
        pointer_class: PointerClass,
    },

    /// Return from function.
    Return { value: mir::Value },

    /// Yield from a coroutine.
    Yield {
        value: mir::Value,
        source: mir::Value,
        resume_point: engine::ResumePointId,
    },

    /// Unconditional jump.
    Jump { target: u32, moves: MoveRange },

    /// Conditional branch.
    Branch {
        condition: mir::Value,
        then_target: u32,
        then_moves: MoveRange,
        else_target: u32,
        else_moves: MoveRange,
    },

    /// Semantic check with explicit success and failure edges.
    Check {
        constraint: CheckId,
        then_target: u32,
        then_moves: MoveRange,
        else_target: u32,
        else_moves: MoveRange,
    },

    /// Fused compare and branch.
    CompareAndBranch {
        left: mir::Value,
        right: mir::Value,
        operator: mir::BinaryOperator,
        then_target: u32,
        then_moves: MoveRange,
        else_target: u32,
        else_moves: MoveRange,
    },

    /// Switch on integer.
    Switch {
        value: mir::Value,
        cases: SwitchCasesId,
        default_target: u32,
        default_moves: MoveRange,
    },

    /// Switch via dense jump table.
    SwitchTable {
        value: mir::Value,
        table: SwitchTableId,
        default_target: u32,
        default_moves: MoveRange,
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
        moves: MoveRange,
    },

    /// Tail call to the current function.
    TailCallSelf {
        entry: u32,
        arguments: ArgumentRange,
    },

    /// Virtual tail call.
    TailCallVirtual {
        receiver: mir::Value,
        table_field: Option<FieldAccessId>,
        method_index: u32,
        arguments: ArgumentRange,
    },

    /// Interface tail call.
    TailCallInterface {
        receiver: mir::Value,
        table_field: Option<FieldAccessId>,
        method_index: u32,
        arguments: ArgumentRange,
    },

    /// Indirect tail call.
    TailCallIndirect {
        callee: mir::Value,
        arguments: ArgumentRange,
    },
}

// operands should fit in 48 bytes
const _: () = assert!(std::mem::size_of::<Operands>() <= 48);
