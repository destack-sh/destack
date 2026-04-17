use std::fmt;

use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::Error;
use destack_heap::{ReferenceMeta, StorageLayoutId, Value};

use super::{ArgumentRange, CallTarget, CopyRange, SwitchRange};

/// Control transfer requested by one lowered instruction.
#[derive(Debug)]
pub(crate) enum Transfer {
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
        /// Lowered or imported call target.
        target: CallTarget,
        /// Destination for return value.
        destination: mir::Value,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Call another function and branch on normal or unwind completion.
    CallBranch {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Value>,
        /// The normal continuation resume point.
        normal_resume_point: engine::ResumePointId,
        /// The unwind continuation resume point.
        unwind_resume_point: engine::ResumePointId,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional function environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Value,
        /// The semantic resume point used by this yield.
        resume_point: engine::ResumePointId,
    },
    /// Throw one managed exception value.
    Throw(Value),
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
    /// Dispatch operation for `composite`.
    Composite,
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
    /// Dispatch operation for `check`.
    Check,
    /// Dispatch operation for `call`.
    Call,
    /// Dispatch operation for `call.branch`.
    CallBranch,
    /// Dispatch operation for `call_indirect`.
    CallIndirect,
    /// Dispatch operation for `call_indirect.branch`.
    CallIndirectBranch,
    /// Dispatch operation for `call_interface`.
    CallInterface,
    /// Dispatch operation for `call_interface.branch`.
    CallInterfaceBranch,
    /// Dispatch operation for `call_virtual`.
    CallVirtual,
    /// Dispatch operation for `call_virtual.branch`.
    CallVirtualBranch,
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
    /// Dispatch operation for `copy`.
    Copy,
    /// Dispatch operation for `index_select`.
    IndexSelect,
    /// Dispatch operation for `select_by_index`.
    SelectByIndex,
    /// Dispatch operation for `element_addr`.
    ElementAddr,
    /// Dispatch operation for `element_addr_composite`.
    ElementAddrComposite,
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
    /// Dispatch operation for `element_load_composite`.
    ElementLoadComposite,
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
    /// Dispatch operation for `element_store_composite`.
    ElementStoreComposite,
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
    /// Dispatch operation for `field_addr_composite`.
    FieldAddrComposite,
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
    /// Dispatch operation for `field_load_composite`.
    FieldLoadComposite,
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
    /// Dispatch operation for `field_store_composite`.
    FieldStoreComposite,
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
    /// Dispatch operation for `function_bind`.
    FunctionBind,
    /// Dispatch operation for `function_environment`.
    FunctionEnvironment,
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
    /// Dispatch operation for `dispose`.
    Dispose,
    /// Dispatch operation for `dispose_async`.
    AsyncDispose,
    /// Dispatch operation for `drop`.
    Drop,
    /// Dispatch operation for `drop_async`.
    AsyncDrop,
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
    /// Dispatch operation for `throw`.
    Throw,
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

/// One precomputed field access descriptor.
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

/// One precomputed element access descriptor.
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

/// One precomputed typed pointee access descriptor.
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

/// Decoded instruction data.
#[derive(Clone, Debug)]
pub(crate) enum InstructionData {
    /// Load constant.
    Const { dest: mir::Value, value: ConstValue },

    /// Copy one value.
    Copy {
        dest: mir::Value,
        source: mir::Value,
    },

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
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Virtual method call terminator with explicit normal and unwind continuations.
    CallVirtualBranch {
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
        normal_resume_point: engine::ResumePointId,
        unwind_resume_point: engine::ResumePointId,
    },

    /// Interface method call.
    CallInterface {
        dest: mir::Value,
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
        slot_id: u32,
        arguments: ArgumentRange,
    },

    /// Interface method call terminator with explicit normal and unwind continuations.
    CallInterfaceBranch {
        receiver: mir::Value,
        managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
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

    /// Load global constant.
    GlobalConst { dest: mir::Value, global: u32 },

    /// Get a function pointer.
    FunctionAddr { dest: mir::Value, function: u32 },

    /// Bind one environment to a function value.
    FunctionBind {
        dest: mir::Value,
        function: u32,
        environment: mir::Value,
    },

    /// Load the function environment pointer.
    FunctionEnvironment { dest: mir::Value },

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
    },

    /// Select one value from a fixed list by runtime index.
    IndexSelect {
        dest: mir::Value,
        index: mir::Value,
        elements: ArgumentRange,
    },

    /// Select between two values by comparing one runtime index to one constant case.
    SelectByIndex {
        dest: mir::Value,
        index: mir::Value,
        match_index: u64,
        then_value: mir::Value,
        else_value: mir::Value,
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

    /// Allocate managed memory.
    ManagedAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
        storage_type: mir::LocalNodeId<mir::Type>,
        layout_id: Option<StorageLayoutId>,
        byte_len: usize,
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
        byte_len: usize,
    },

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Run explicit synchronous cleanup.
    Dispose { value: mir::Value },

    /// Run explicit asynchronous cleanup.
    AsyncDispose { value: mir::Value },

    /// End ownership synchronously.
    Drop { value: mir::Value },

    /// End ownership asynchronously.
    AsyncDrop { value: mir::Value },

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
        arguments: ArgumentRange,
    },
}

impl InstructionData {
    /// Return a short opcode label for instruction profiling.
    #[cfg(feature = "stats")]
    pub(crate) fn opcode_name(&self) -> &'static str {
        match self {
            InstructionData::Const { .. } => "const",
            InstructionData::Copy { .. } => "copy",
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
            InstructionData::CallBranch { .. } => "call_branch",
            InstructionData::CallVirtual { .. } => "call_virtual",
            InstructionData::CallVirtualBranch { .. } => "call_virtual_branch",
            InstructionData::CallInterface { .. } => "call_interface",
            InstructionData::CallInterfaceBranch { .. } => "call_interface_branch",
            InstructionData::CallIndirect { .. } => "call_indirect",
            InstructionData::CallIndirectBranch { .. } => "call_indirect_branch",
            InstructionData::LocalGet { .. } => "local_get",
            InstructionData::LocalAddr { .. } => "local_addr",
            InstructionData::LocalSet { .. } => "local_set",
            InstructionData::GlobalAddr { .. } => "global_addr",
            InstructionData::GlobalConst { .. } => "global_const",
            InstructionData::FunctionAddr { .. } => "function_addr",
            InstructionData::FunctionBind { .. } => "function_bind",
            InstructionData::FunctionEnvironment { .. } => "function_environment",
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
            InstructionData::IndexSelect { .. } => "index_select",
            InstructionData::SelectByIndex { .. } => "select_by_index",
            InstructionData::ElementAddr { .. } => "element_addr",
            InstructionData::ElementLoad { .. } => "element_load",
            InstructionData::ElementSet { .. } => "element_set",
            InstructionData::ElementStore { .. } => "element_store",
            InstructionData::Composite { .. } => "composite",
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
            InstructionData::Dispose { .. } => "dispose",
            InstructionData::AsyncDispose { .. } => "dispose_async",
            InstructionData::Drop { .. } => "drop",
            InstructionData::AsyncDrop { .. } => "drop_async",
            InstructionData::StackAlloc { .. } => "stack_alloc",
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
            InstructionData::Check { .. } => "check",
            InstructionData::CompareAndBranch { .. } => "compare_and_branch",
            InstructionData::CompareAndBranchConst { .. } => "compare_and_branch_const",
            InstructionData::Switch { .. } => "switch",
            InstructionData::SwitchTable { .. } => "switch_table",
            InstructionData::Throw { .. } => "throw",
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
