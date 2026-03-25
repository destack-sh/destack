use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

use destack_mir as mir;

use crate::diagnostic::Error;
use destack_heap::{LayoutId, ReferenceMap, ReferenceMeta, Value};

use super::{ArgumentRange, CopyRange, Function, SwitchRange};

/// Control flow actions that exit the tail-call chain.
#[derive(Debug)]
pub(crate) enum ControlFlow {
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
        /// Lowered function index when available.
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
        /// Lowered function index when available.
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

/// Interpreter dispatch operation for one lowered instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstructionOperation {
    /// Dispatch operation for `add_const_int`.
    AddConstInt,
    /// Dispatch operation for `add_const_uint`.
    AddConstUint,
    /// Dispatch operation for `add_int`.
    AddInt,
    /// Dispatch operation for `add_uint`.
    AddUint,
    /// Dispatch operation for `aggregate`.
    Aggregate,
    /// Dispatch operation for `and_int`.
    AndInt,
    /// Dispatch operation for `and_uint`.
    AndUint,
    /// Dispatch operation for `assume`.
    Assume,
    /// Dispatch operation for `atomic_compare_exchange`.
    AtomicCompareExchange,
    /// Dispatch operation for `atomic_fence`.
    AtomicFence,
    /// Dispatch operation for `atomic_load`.
    AtomicLoad,
    /// Dispatch operation for `atomic_rmw`.
    AtomicRmw,
    /// Dispatch operation for `atomic_store`.
    AtomicStore,
    /// Dispatch operation for `barrier`.
    Barrier,
    /// Dispatch operation for `binary`.
    Binary,
    /// Dispatch operation for `binary_bool`.
    BinaryBool,
    /// Dispatch operation for `binary_const_right`.
    BinaryConstRight,
    /// Dispatch operation for `binary_elementwise`.
    BinaryElementwise,
    /// Dispatch operation for `binary_float32`.
    BinaryFloat32,
    /// Dispatch operation for `binary_float64`.
    BinaryFloat64,
    /// Dispatch operation for `binary_int`.
    BinaryInt,
    /// Dispatch operation for `binary_uint`.
    BinaryUint,
    /// Dispatch operation for `branch`.
    Branch,
    /// Dispatch operation for `branch_bool`.
    BranchBool,
    /// Dispatch operation for `call`.
    Call,
    /// Dispatch operation for `call_indirect`.
    CallIndirect,
    /// Dispatch operation for `call_interface`.
    CallInterface,
    /// Dispatch operation for `call_virtual`.
    CallVirtual,
    /// Dispatch operation for `cast`.
    Cast,
    /// Dispatch operation for `compare_and_branch`.
    CompareAndBranch,
    /// Dispatch operation for `compare_and_branch_const`.
    CompareAndBranchConst,
    /// Dispatch operation for `compare_and_branch_const_float`.
    CompareAndBranchConstFloat,
    /// Dispatch operation for `compare_and_branch_const_int`.
    CompareAndBranchConstInt,
    /// Dispatch operation for `compare_and_branch_const_uint`.
    CompareAndBranchConstUint,
    /// Dispatch operation for `compare_and_branch_float`.
    CompareAndBranchFloat,
    /// Dispatch operation for `compare_and_branch_int`.
    CompareAndBranchInt,
    /// Dispatch operation for `compare_and_branch_uint`.
    CompareAndBranchUint,
    /// Dispatch operation for `const`.
    Const,
    /// Dispatch operation for `element_addr`.
    ElementAddr,
    /// Dispatch operation for `element_addr_aggregate`.
    ElementAddrAggregate,
    /// Dispatch operation for `element_addr_global`.
    ElementAddrGlobal,
    /// Dispatch operation for `element_addr_managed`.
    ElementAddrManaged,
    /// Dispatch operation for `element_addr_raw`.
    ElementAddrRaw,
    /// Dispatch operation for `element_addr_stack`.
    ElementAddrStack,
    /// Dispatch operation for `element_get`.
    ElementGet,
    /// Dispatch operation for `element_load`.
    ElementLoad,
    /// Dispatch operation for `element_load_aggregate`.
    ElementLoadAggregate,
    /// Dispatch operation for `element_load_global`.
    ElementLoadGlobal,
    /// Dispatch operation for `element_load_managed`.
    ElementLoadManaged,
    /// Dispatch operation for `element_load_raw`.
    ElementLoadRaw,
    /// Dispatch operation for `element_load_stack`.
    ElementLoadStack,
    /// Dispatch operation for `element_set`.
    ElementSet,
    /// Dispatch operation for `element_store`.
    ElementStore,
    /// Dispatch operation for `element_store_aggregate`.
    ElementStoreAggregate,
    /// Dispatch operation for `element_store_global`.
    ElementStoreGlobal,
    /// Dispatch operation for `element_store_managed`.
    ElementStoreManaged,
    /// Dispatch operation for `element_store_raw`.
    ElementStoreRaw,
    /// Dispatch operation for `element_store_stack`.
    ElementStoreStack,
    /// Dispatch operation for `eq_const_int`.
    EqConstInt,
    /// Dispatch operation for `eq_int`.
    EqInt,
    /// Dispatch operation for `field_addr`.
    FieldAddr,
    /// Dispatch operation for `field_addr_aggregate`.
    FieldAddrAggregate,
    /// Dispatch operation for `field_addr_global`.
    FieldAddrGlobal,
    /// Dispatch operation for `field_addr_managed`.
    FieldAddrManaged,
    /// Dispatch operation for `field_addr_raw`.
    FieldAddrRaw,
    /// Dispatch operation for `field_addr_stack`.
    FieldAddrStack,
    /// Dispatch operation for `field_get`.
    FieldGet,
    /// Dispatch operation for `field_get_inline`.
    FieldGetInline,
    /// Dispatch operation for `field_load`.
    FieldLoad,
    /// Dispatch operation for `field_load_aggregate`.
    FieldLoadAggregate,
    /// Dispatch operation for `field_load_global`.
    FieldLoadGlobal,
    /// Dispatch operation for `field_load_managed`.
    FieldLoadManaged,
    /// Dispatch operation for `field_load_raw`.
    FieldLoadRaw,
    /// Dispatch operation for `field_load_stack`.
    FieldLoadStack,
    /// Dispatch operation for `field_set`.
    FieldSet,
    /// Dispatch operation for `field_store`.
    FieldStore,
    /// Dispatch operation for `field_store_aggregate`.
    FieldStoreAggregate,
    /// Dispatch operation for `field_store_global`.
    FieldStoreGlobal,
    /// Dispatch operation for `field_store_inline`.
    FieldStoreInline,
    /// Dispatch operation for `field_store_managed`.
    FieldStoreManaged,
    /// Dispatch operation for `field_store_raw`.
    FieldStoreRaw,
    /// Dispatch operation for `field_store_stack`.
    FieldStoreStack,
    /// Dispatch operation for `function_addr`.
    FunctionAddr,
    /// Dispatch operation for `function_env`.
    FunctionEnv,
    /// Dispatch operation for `ge_const_int`.
    GeConstInt,
    /// Dispatch operation for `ge_const_uint`.
    GeConstUint,
    /// Dispatch operation for `ge_int`.
    GeInt,
    /// Dispatch operation for `ge_uint`.
    GeUint,
    /// Dispatch operation for `global_addr`.
    GlobalAddr,
    /// Dispatch operation for `global_const`.
    GlobalConst,
    /// Dispatch operation for `global_load`.
    GlobalLoad,
    /// Dispatch operation for `global_store`.
    GlobalStore,
    /// Dispatch operation for `gt_const_int`.
    GtConstInt,
    /// Dispatch operation for `gt_const_uint`.
    GtConstUint,
    /// Dispatch operation for `gt_int`.
    GtInt,
    /// Dispatch operation for `gt_uint`.
    GtUint,
    /// Dispatch operation for `intrinsic`.
    Intrinsic,
    /// Dispatch operation for `jump`.
    Jump,
    /// Dispatch operation for `le_const_int`.
    LeConstInt,
    /// Dispatch operation for `le_const_uint`.
    LeConstUint,
    /// Dispatch operation for `le_int`.
    LeInt,
    /// Dispatch operation for `le_uint`.
    LeUint,
    /// Dispatch operation for `load`.
    Load,
    /// Dispatch operation for `load_global`.
    LoadGlobal,
    /// Dispatch operation for `load_local`.
    LoadLocal,
    /// Dispatch operation for `load_managed`.
    LoadManaged,
    /// Dispatch operation for `load_raw`.
    LoadRaw,
    /// Dispatch operation for `load_stack`.
    LoadStack,
    /// Dispatch operation for `local_addr`.
    LocalAddr,
    /// Dispatch operation for `local_get`.
    LocalGet,
    /// Dispatch operation for `local_set`.
    LocalSet,
    /// Dispatch operation for `lt_const_int`.
    LtConstInt,
    /// Dispatch operation for `lt_const_uint`.
    LtConstUint,
    /// Dispatch operation for `lt_int`.
    LtInt,
    /// Dispatch operation for `lt_uint`.
    LtUint,
    /// Dispatch operation for `managed_alloc`.
    ManagedAlloc,
    /// Dispatch operation for `managed_alloc_array`.
    ManagedAllocArray,
    /// Dispatch operation for `mul_const_int`.
    MulConstInt,
    /// Dispatch operation for `mul_const_uint`.
    MulConstUint,
    /// Dispatch operation for `mul_int`.
    MulInt,
    /// Dispatch operation for `mul_uint`.
    MulUint,
    /// Dispatch operation for `ne_const_int`.
    NeConstInt,
    /// Dispatch operation for `ne_int`.
    NeInt,
    /// Dispatch operation for `or_int`.
    OrInt,
    /// Dispatch operation for `or_uint`.
    OrUint,
    /// Dispatch operation for `raw_alloc`.
    RawAlloc,
    /// Dispatch operation for `raw_drop`.
    RawDrop,
    /// Dispatch operation for `raw_free`.
    RawFree,
    /// Dispatch operation for `return`.
    Return,
    /// Dispatch operation for `select`.
    Select,
    /// Dispatch operation for `shl_int`.
    ShlInt,
    /// Dispatch operation for `shl_uint`.
    ShlUint,
    /// Dispatch operation for `shr_int`.
    ShrInt,
    /// Dispatch operation for `shr_uint`.
    ShrUint,
    /// Dispatch operation for `stack_alloc`.
    StackAlloc,
    /// Dispatch operation for `stack_drop`.
    StackDrop,
    /// Dispatch operation for `store`.
    Store,
    /// Dispatch operation for `store_global`.
    StoreGlobal,
    /// Dispatch operation for `store_local`.
    StoreLocal,
    /// Dispatch operation for `store_managed`.
    StoreManaged,
    /// Dispatch operation for `store_raw`.
    StoreRaw,
    /// Dispatch operation for `store_stack`.
    StoreStack,
    /// Dispatch operation for `sub_const_int`.
    SubConstInt,
    /// Dispatch operation for `sub_const_uint`.
    SubConstUint,
    /// Dispatch operation for `sub_int`.
    SubInt,
    /// Dispatch operation for `sub_uint`.
    SubUint,
    /// Dispatch operation for `switch`.
    Switch,
    /// Dispatch operation for `switch_int`.
    SwitchInt,
    /// Dispatch operation for `switch_table`.
    SwitchTable,
    /// Dispatch operation for `switch_table_int`.
    SwitchTableInt,
    /// Dispatch operation for `tail_call`.
    TailCall,
    /// Dispatch operation for `tail_call_indirect`.
    TailCallIndirect,
    /// Dispatch operation for `tail_call_interface`.
    TailCallInterface,
    /// Dispatch operation for `tail_call_self`.
    TailCallSelf,
    /// Dispatch operation for `tail_call_virtual`.
    TailCallVirtual,
    /// Dispatch operation for `tensor_broadcast`.
    TensorBroadcast,
    /// Dispatch operation for `tensor_cast`.
    TensorCast,
    /// Dispatch operation for `tensor_compare`.
    TensorCompare,
    /// Dispatch operation for `tensor_concat`.
    TensorConcat,
    /// Dispatch operation for `tensor_convert`.
    TensorConvert,
    /// Dispatch operation for `tensor_convolution`.
    TensorConvolution,
    /// Dispatch operation for `tensor_copy`.
    TensorCopy,
    /// Dispatch operation for `tensor_dot`.
    TensorDot,
    /// Dispatch operation for `tensor_fill`.
    TensorFill,
    /// Dispatch operation for `tensor_gather`.
    TensorGather,
    /// Dispatch operation for `tensor_load`.
    TensorLoad,
    /// Dispatch operation for `tensor_pad`.
    TensorPad,
    /// Dispatch operation for `tensor_reduce`.
    TensorReduce,
    /// Dispatch operation for `tensor_reshape`.
    TensorReshape,
    /// Dispatch operation for `tensor_scatter`.
    TensorScatter,
    /// Dispatch operation for `tensor_select`.
    TensorSelect,
    /// Dispatch operation for `tensor_slice`.
    TensorSlice,
    /// Dispatch operation for `tensor_store`.
    TensorStore,
    /// Dispatch operation for `tensor_transpose`.
    TensorTranspose,
    /// Dispatch operation for `tensor_view`.
    TensorView,
    /// Dispatch operation for `trap`.
    Trap,
    /// Dispatch operation for `unary`.
    Unary,
    /// Dispatch operation for `unary_bool`.
    UnaryBool,
    /// Dispatch operation for `unary_elementwise`.
    UnaryElementwise,
    /// Dispatch operation for `unary_float32`.
    UnaryFloat32,
    /// Dispatch operation for `unary_float64`.
    UnaryFloat64,
    /// Dispatch operation for `unary_int`.
    UnaryInt,
    /// Dispatch operation for `unary_uint`.
    UnaryUint,
    /// Dispatch operation for `unreachable`.
    Unreachable,
    /// Dispatch operation for `vector_compare`.
    VectorCompare,
    /// Dispatch operation for `vector_convert`.
    VectorConvert,
    /// Dispatch operation for `vector_extract`.
    VectorExtract,
    /// Dispatch operation for `vector_insert`.
    VectorInsert,
    /// Dispatch operation for `vector_reduce`.
    VectorReduce,
    /// Dispatch operation for `vector_select`.
    VectorSelect,
    /// Dispatch operation for `vector_shuffle`.
    VectorShuffle,
    /// Dispatch operation for `vector_splat`.
    VectorSplat,
    /// Dispatch operation for `xor_int`.
    XorInt,
    /// Dispatch operation for `xor_uint`.
    XorUint,
    /// Dispatch operation for `yield`.
    Yield,
}

/// Pre-decoded instruction with one interpreter operation tag.
#[derive(Clone)]
pub(crate) struct Instruction {
    /// The interpreter operation for this instruction.
    pub operation: InstructionOperation,
    /// Decoded data.
    pub data: InstructionData,
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction")
            .field("data", &self.data)
            .finish()
    }
}

/// Constant value payload stored in lowered instructions.
#[derive(Clone, Debug)]
pub(crate) enum ConstValue {
    /// Pre-decoded constant value.
    Value(Value),
}

/// Decoded instruction data.
#[derive(Clone, Debug)]
pub(crate) enum InstructionData {
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

    /// Specialized binary operation.
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

    /// Specialized binary with constant right.
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

    /// Get struct or tuple field.
    FieldGet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
    },

    /// Get struct or tuple field address.
    FieldAddr {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        reference: ReferenceMeta,
        field_count: u32,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Load a field through field address plus load.
    FieldLoad {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        field_count: u32,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    },

    /// Set struct or tuple field.
    FieldSet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    },

    /// Store a field through field address plus store.
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

    /// Load an element through element address plus load.
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

    /// Construct an aggregate from element values.
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
    },

    /// Store an element through element address plus store.
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

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Drop raw memory.
    RawDrop { value: mir::Value },

    /// Allocate stack memory.
    StackAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        slot_count: u32,
    },

    /// Mark stack value lifetime ended.
    StackDrop,

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
        resume_block: u32,
        resume_copies: CopyRange,
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

    /// Unreachable code.
    Unreachable,

    /// Tail call to a function.
    TailCall {
        function: u32,
        callee_index: u32,
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
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface tail call.
    TailCallInterface {
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Indirect tail call.
    TailCallIndirect {
        callee: mir::Value,
        env: Option<mir::Value>,
        arguments: ArgumentRange,
        cached_function: Cell<Option<u32>>,
        cached_ptr: Cell<Option<NonNull<Function>>>,
    },
}

impl InstructionData {
    /// Return a short opcode label for instruction profiling.
    #[cfg(feature = "stats")]
    pub fn opcode_name(&self) -> &'static str {
        match self {
            InstructionData::Const { .. } => "const",
            InstructionData::Binary { .. } => "binary",
            InstructionData::BinaryElementwise { .. } => "binary_elementwise",
            InstructionData::BinarySpecialized { .. } => "binary_specialized",
            InstructionData::BinaryConstRight { .. } => "binary_const_right",
            InstructionData::BinaryConstRightSpecialized { .. } => "binary_const_right_specialized",
            InstructionData::Unary { .. } => "unary",
            InstructionData::UnaryElementwise { .. } => "unary_elementwise",
            InstructionData::Cast { .. } => "cast",
            InstructionData::Select { .. } => "select",
            InstructionData::Call { .. } => "call",
            InstructionData::CallVirtual { .. } => "call_virtual",
            InstructionData::CallInterface { .. } => "call_interface",
            InstructionData::CallIndirect { .. } => "call_indirect",
            InstructionData::LocalGet { .. } => "local_get",
            InstructionData::LocalAddr { .. } => "local_addr",
            InstructionData::LocalSet { .. } => "local_set",
            InstructionData::GlobalAddr { .. } => "global_addr",
            InstructionData::GlobalConst { .. } => "global_const",
            InstructionData::FunctionAddr { .. } => "function_addr",
            InstructionData::FunctionEnv { .. } => "function_env",
            InstructionData::GlobalLoad { .. } => "global_load",
            InstructionData::GlobalStore { .. } => "global_store",
            InstructionData::Load { .. } => "load",
            InstructionData::Store { .. } => "store",
            InstructionData::FieldGet { .. } => "field_get",
            InstructionData::FieldAddr { .. } => "field_addr",
            InstructionData::FieldLoad { .. } => "field_load",
            InstructionData::FieldSet { .. } => "field_set",
            InstructionData::FieldStore { .. } => "field_store",
            InstructionData::ElementGet { .. } => "element_get",
            InstructionData::ElementAddr { .. } => "element_addr",
            InstructionData::ElementLoad { .. } => "element_load",
            InstructionData::ElementSet { .. } => "element_set",
            InstructionData::ElementStore { .. } => "element_store",
            InstructionData::Aggregate { .. } => "aggregate",
            InstructionData::VectorSplat { .. } => "vector_splat",
            InstructionData::VectorExtract { .. } => "vector_extract",
            InstructionData::VectorInsert { .. } => "vector_insert",
            InstructionData::VectorShuffle { .. } => "vector_shuffle",
            InstructionData::VectorSelect { .. } => "vector_select",
            InstructionData::VectorReduce { .. } => "vector_reduce",
            InstructionData::VectorCompare { .. } => "vector_compare",
            InstructionData::VectorConvert { .. } => "vector_convert",
            InstructionData::TensorLoad { .. } => "tensor_load",
            InstructionData::TensorStore { .. } => "tensor_store",
            InstructionData::TensorFill { .. } => "tensor_fill",
            InstructionData::TensorCopy { .. } => "tensor_copy",
            InstructionData::TensorReshape { .. } => "tensor_reshape",
            InstructionData::TensorBroadcast { .. } => "tensor_broadcast",
            InstructionData::TensorTranspose { .. } => "tensor_transpose",
            InstructionData::TensorCast { .. } => "tensor_cast",
            InstructionData::TensorView { .. } => "tensor_view",
            InstructionData::TensorSlice { .. } => "tensor_slice",
            InstructionData::TensorPad { .. } => "tensor_pad",
            InstructionData::TensorConcat { .. } => "tensor_concat",
            InstructionData::TensorReduce { .. } => "tensor_reduce",
            InstructionData::TensorDot { .. } => "tensor_dot",
            InstructionData::TensorConvolution { .. } => "tensor_convolution",
            InstructionData::TensorGather { .. } => "tensor_gather",
            InstructionData::TensorScatter { .. } => "tensor_scatter",
            InstructionData::TensorCompare { .. } => "tensor_compare",
            InstructionData::TensorSelect { .. } => "tensor_select",
            InstructionData::TensorConvert { .. } => "tensor_convert",
            InstructionData::ManagedAlloc { .. } => "managed_alloc",
            InstructionData::ManagedAllocArray { .. } => "managed_alloc_array",
            InstructionData::RawAlloc { .. } => "raw_alloc",
            InstructionData::RawFree { .. } => "raw_free",
            InstructionData::RawDrop { .. } => "raw_drop",
            InstructionData::StackAlloc { .. } => "stack_alloc",
            InstructionData::StackDrop => "stack_drop",
            InstructionData::Assume => "assume",
            InstructionData::Intrinsic { .. } => "intrinsic",
            InstructionData::AtomicLoad { .. } => "atomic_load",
            InstructionData::AtomicStore { .. } => "atomic_store",
            InstructionData::AtomicCompareExchange { .. } => "atomic_compare_exchange",
            InstructionData::AtomicRmw { .. } => "atomic_rmw",
            InstructionData::AtomicFence => "atomic_fence",
            InstructionData::Barrier => "barrier",
            InstructionData::Return { .. } => "return",
            InstructionData::Yield { .. } => "yield",
            InstructionData::Jump { .. } => "jump",
            InstructionData::Branch { .. } => "branch",
            InstructionData::CompareAndBranch { .. } => "compare_and_branch",
            InstructionData::CompareAndBranchConst { .. } => "compare_and_branch_const",
            InstructionData::Switch { .. } => "switch",
            InstructionData::SwitchTable { .. } => "switch_table",
            InstructionData::Trap { .. } => "trap",
            InstructionData::Unreachable => "unreachable",
            InstructionData::TailCall { .. } => "tail_call",
            InstructionData::TailCallSelf { .. } => "tail_call_self",
            InstructionData::TailCallVirtual { .. } => "tail_call_virtual",
            InstructionData::TailCallInterface { .. } => "tail_call_interface",
            InstructionData::TailCallIndirect { .. } => "tail_call_indirect",
        }
    }
}
