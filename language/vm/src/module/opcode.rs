/// Operation code for one decoded module instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opcode {
    /// The `add_const_int` opcode.
    AddConstInt,
    /// The `add_const_uint` opcode.
    AddConstUint,
    /// The `add_int` opcode.
    AddInt,
    /// The `add_uint` opcode.
    AddUint,
    /// The `composite` opcode.
    Composite,
    /// The `and_int` opcode.
    AndInt,
    /// The `and_uint` opcode.
    AndUint,
    /// The `assume` opcode.
    Assume,
    /// The `atomic_compare_exchange` opcode.
    AtomicCompareExchange,
    /// The `atomic_fence` opcode.
    AtomicFence,
    /// The `atomic_load` opcode.
    AtomicLoad,
    /// The `atomic_rmw` opcode.
    AtomicRmw,
    /// The `atomic_store` opcode.
    AtomicStore,
    /// The `barrier` opcode.
    Barrier,
    /// The `binary` opcode.
    Binary,
    /// The `binary_bool` opcode.
    BinaryBool,
    /// The `binary_const_right` opcode.
    BinaryConstRight,
    /// The `binary_elementwise` opcode.
    BinaryElementwise,
    /// The `binary_float32` opcode.
    BinaryFloat32,
    /// The `binary_float64` opcode.
    BinaryFloat64,
    /// The `binary_int` opcode.
    BinaryInt,
    /// The `binary_uint` opcode.
    BinaryUint,
    /// The `branch` opcode.
    Branch,
    /// The `branch_bool` opcode.
    BranchBool,
    /// The `check` opcode.
    Check,
    /// The `call` opcode.
    Call,
    /// The `call.branch` opcode.
    CallBranch,
    /// The `call_indirect` opcode.
    CallIndirect,
    /// The `call_indirect.branch` opcode.
    CallIndirectBranch,
    /// The `call_interface` opcode.
    CallInterface,
    /// The `call_interface.branch` opcode.
    CallInterfaceBranch,
    /// The `call_virtual` opcode.
    CallVirtual,
    /// The `call_virtual.branch` opcode.
    CallVirtualBranch,
    /// The `cast` opcode.
    Cast,
    /// The `compare_and_branch` opcode.
    CompareAndBranch,
    /// The `compare_and_branch_const` opcode.
    CompareAndBranchConst,
    /// The `compare_and_branch_const_float` opcode.
    CompareAndBranchConstFloat,
    /// The `compare_and_branch_const_int` opcode.
    CompareAndBranchConstInt,
    /// The `compare_and_branch_const_uint` opcode.
    CompareAndBranchConstUint,
    /// The `compare_and_branch_float` opcode.
    CompareAndBranchFloat,
    /// The `compare_and_branch_int` opcode.
    CompareAndBranchInt,
    /// The `compare_and_branch_uint` opcode.
    CompareAndBranchUint,
    /// The `const` opcode.
    Const,
    /// The `element_addr` opcode.
    ElementAddr,
    /// The `element_addr_global` opcode.
    ElementAddrGlobal,
    /// The `element_addr_heap` opcode.
    ElementAddrHeap,
    /// The `element_addr_raw` opcode.
    ElementAddrRaw,
    /// The `element_addr_stack` opcode.
    ElementAddrStack,
    /// The `element_get` opcode.
    ElementGet,
    /// The `element_load` opcode.
    ElementLoad,
    /// The `element_load_global` opcode.
    ElementLoadGlobal,
    /// The `element_load_heap` opcode.
    ElementLoadHeap,
    /// The `element_load_raw` opcode.
    ElementLoadRaw,
    /// The `element_load_stack` opcode.
    ElementLoadStack,
    /// The `element_set` opcode.
    ElementSet,
    /// The `element_store` opcode.
    ElementStore,
    /// The `element_store_global` opcode.
    ElementStoreGlobal,
    /// The `element_store_heap` opcode.
    ElementStoreHeap,
    /// The `element_store_raw` opcode.
    ElementStoreRaw,
    /// The `element_store_stack` opcode.
    ElementStoreStack,
    /// The `eq_const_int` opcode.
    EqConstInt,
    /// The `eq_int` opcode.
    EqInt,
    /// The `field_addr` opcode.
    FieldAddr,
    /// The `field_addr_global` opcode.
    FieldAddrGlobal,
    /// The `field_addr_heap` opcode.
    FieldAddrHeap,
    /// The `field_addr_raw` opcode.
    FieldAddrRaw,
    /// The `field_addr_stack` opcode.
    FieldAddrStack,
    /// The `field_get` opcode.
    FieldGet,
    /// The `field_load` opcode.
    FieldLoad,
    /// The `field_load_global` opcode.
    FieldLoadGlobal,
    /// The `field_load_heap` opcode.
    FieldLoadHeap,
    /// The `field_load_raw` opcode.
    FieldLoadRaw,
    /// The `field_load_stack` opcode.
    FieldLoadStack,
    /// The `field_set` opcode.
    FieldSet,
    /// The `field_store` opcode.
    FieldStore,
    /// The `field_store_global` opcode.
    FieldStoreGlobal,
    /// The `field_store_heap` opcode.
    FieldStoreHeap,
    /// The `field_store_raw` opcode.
    FieldStoreRaw,
    /// The `field_store_stack` opcode.
    FieldStoreStack,
    /// The `function_addr` opcode.
    FunctionAddr,
    /// The `callable_bind` opcode.
    CallableBind,
    /// The `callable_environment` opcode.
    CallableEnvironment,
    /// The `ge_const_int` opcode.
    GeConstInt,
    /// The `ge_const_uint` opcode.
    GeConstUint,
    /// The `ge_int` opcode.
    GeInt,
    /// The `ge_uint` opcode.
    GeUint,
    /// The `global_addr` opcode.
    GlobalAddr,
    /// The `global_load` opcode.
    GlobalLoad,
    /// The `global_store` opcode.
    GlobalStore,
    /// The `gt_const_int` opcode.
    GtConstInt,
    /// The `gt_const_uint` opcode.
    GtConstUint,
    /// The `gt_int` opcode.
    GtInt,
    /// The `gt_uint` opcode.
    GtUint,
    /// The `intrinsic` opcode.
    Intrinsic,
    /// The `jump` opcode.
    Jump,
    /// The `le_const_int` opcode.
    LeConstInt,
    /// The `le_const_uint` opcode.
    LeConstUint,
    /// The `le_int` opcode.
    LeInt,
    /// The `le_uint` opcode.
    LeUint,
    /// The `load` opcode.
    Load,
    /// The `load_global` opcode.
    LoadGlobal,
    /// The `load_frame` opcode.
    LoadFrame,
    /// The `load_heap` opcode.
    LoadHeap,
    /// The `load_raw` opcode.
    LoadRaw,
    /// The `load_stack` opcode.
    LoadStack,
    /// The `local_addr` opcode.
    LocalAddr,
    /// The `local_get` opcode.
    LocalGet,
    /// The `local_set` opcode.
    LocalSet,
    /// The `lt_const_int` opcode.
    LtConstInt,
    /// The `lt_const_uint` opcode.
    LtConstUint,
    /// The `lt_int` opcode.
    LtInt,
    /// The `lt_uint` opcode.
    LtUint,
    /// The `new` opcode.
    New,
    /// The `new.slice` opcode.
    NewSlice,
    /// The `mul_const_int` opcode.
    MulConstInt,
    /// The `mul_const_uint` opcode.
    MulConstUint,
    /// The `mul_int` opcode.
    MulInt,
    /// The `mul_uint` opcode.
    MulUint,
    /// The `ne_const_int` opcode.
    NeConstInt,
    /// The `ne_int` opcode.
    NeInt,
    /// The `or_int` opcode.
    OrInt,
    /// The `or_uint` opcode.
    OrUint,
    /// The `raw_alloc` opcode.
    RawAlloc,
    /// The `dispose` opcode.
    Dispose,
    /// The `dispose_async` opcode.
    AsyncDispose,
    /// The `pin` opcode.
    Pin,
    /// The `unpin` opcode.
    Unpin,
    /// The `drop` opcode.
    Drop,
    /// The `raw_free` opcode.
    RawFree,
    /// The `return` opcode.
    Return,
    /// The `select` opcode.
    Select,
    /// The `shl_int` opcode.
    ShlInt,
    /// The `shl_uint` opcode.
    ShlUint,
    /// The `shr_int` opcode.
    ShrInt,
    /// The `shr_uint` opcode.
    ShrUint,
    /// The `stack_alloc` opcode.
    StackAlloc,
    /// The `store` opcode.
    Store,
    /// The `store_global` opcode.
    StoreGlobal,
    /// The `store_frame` opcode.
    StoreFrame,
    /// The `store_heap` opcode.
    StoreHeap,
    /// The `store_raw` opcode.
    StoreRaw,
    /// The `store_stack` opcode.
    StoreStack,
    /// The `sub_const_int` opcode.
    SubConstInt,
    /// The `sub_const_uint` opcode.
    SubConstUint,
    /// The `sub_int` opcode.
    SubInt,
    /// The `sub_uint` opcode.
    SubUint,
    /// The `switch` opcode.
    Switch,
    /// The `switch_int` opcode.
    SwitchInt,
    /// The `switch_table` opcode.
    SwitchTable,
    /// The `switch_table_int` opcode.
    SwitchTableInt,
    /// The `tail_call` opcode.
    TailCall,
    /// The `tail_call_indirect` opcode.
    TailCallIndirect,
    /// The `tail_call_interface` opcode.
    TailCallInterface,
    /// The `tail_call_self` opcode.
    TailCallSelf,
    /// The `tail_call_virtual` opcode.
    TailCallVirtual,
    /// The `tensor_broadcast` opcode.
    TensorBroadcast,
    /// The `tensor_cast` opcode.
    TensorCast,
    /// The `tensor_compare` opcode.
    TensorCompare,
    /// The `tensor_concat` opcode.
    TensorConcat,
    /// The `tensor_convert` opcode.
    TensorConvert,
    /// The `tensor_convolution` opcode.
    TensorConvolution,
    /// The `tensor_copy` opcode.
    TensorCopy,
    /// The `tensor_dot` opcode.
    TensorDot,
    /// The `tensor_fill` opcode.
    TensorFill,
    /// The `tensor_gather` opcode.
    TensorGather,
    /// The `tensor_load` opcode.
    TensorLoad,
    /// The `tensor_splat` opcode.
    TensorSplat,
    /// The `tensor_extract` opcode.
    TensorExtract,
    /// The `tensor_pad` opcode.
    TensorPad,
    /// The `tensor_reduce` opcode.
    TensorReduce,
    /// The `tensor_reshape` opcode.
    TensorReshape,
    /// The `tensor_scatter` opcode.
    TensorScatter,
    /// The `tensor_select` opcode.
    TensorSelect,
    /// The `tensor_slice` opcode.
    TensorSlice,
    /// The `tensor_store` opcode.
    TensorStore,
    /// The `tensor_transpose` opcode.
    TensorTranspose,
    /// The `tensor_view` opcode.
    TensorView,
    /// The `trap` opcode.
    Trap,
    /// The `throw` opcode.
    Throw,
    /// The `unary` opcode.
    Unary,
    /// The `unary_bool` opcode.
    UnaryBool,
    /// The `unary_elementwise` opcode.
    UnaryElementwise,
    /// The `unary_float32` opcode.
    UnaryFloat32,
    /// The `unary_float64` opcode.
    UnaryFloat64,
    /// The `unary_int` opcode.
    UnaryInt,
    /// The `unary_uint` opcode.
    UnaryUint,
    /// The `unreachable` opcode.
    Unreachable,
    /// The `vector_compare` opcode.
    VectorCompare,
    /// The `vector_convert` opcode.
    VectorConvert,
    /// The `vector_extract` opcode.
    VectorExtract,
    /// The `vector_insert` opcode.
    VectorInsert,
    /// The `vector_reduce` opcode.
    VectorReduce,
    /// The `vector_select` opcode.
    VectorSelect,
    /// The `vector_shuffle` opcode.
    VectorShuffle,
    /// The `vector_splat` opcode.
    VectorSplat,
    /// The `xor_int` opcode.
    XorInt,
    /// The `xor_uint` opcode.
    XorUint,
    /// The `yield` opcode.
    Yield,
}

impl Opcode {
    /// Return a short opcode label for instruction profiling.
    #[cfg(feature = "stats")]
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Opcode::Const => "const",
            Opcode::Binary => "binary",
            Opcode::BinaryElementwise => "binary_elementwise",
            Opcode::BinaryConstRight => "binary_const_right",
            Opcode::Unary => "unary",
            Opcode::UnaryElementwise => "unary_elementwise",
            Opcode::Cast => "cast",
            Opcode::Select => "select",
            Opcode::Call => "call",
            Opcode::CallBranch => "call_branch",
            Opcode::CallVirtual => "call_virtual",
            Opcode::CallVirtualBranch => "call_virtual_branch",
            Opcode::CallInterface => "call_interface",
            Opcode::CallInterfaceBranch => "call_interface_branch",
            Opcode::CallIndirect => "call_indirect",
            Opcode::CallIndirectBranch => "call_indirect_branch",
            Opcode::LocalGet => "local_get",
            Opcode::LocalAddr => "local_addr",
            Opcode::LocalSet => "local_set",
            Opcode::GlobalAddr => "global_addr",
            Opcode::FunctionAddr => "function_addr",
            Opcode::CallableBind => "callable_bind",
            Opcode::CallableEnvironment => "callable_environment",
            Opcode::GlobalLoad => "global_load",
            Opcode::GlobalStore => "global_store",
            Opcode::Load => "load",
            Opcode::Store => "store",
            Opcode::FieldGet => "field_get",
            Opcode::FieldAddr => "field_addr",
            Opcode::FieldLoad => "field_load",
            Opcode::FieldSet => "field_set",
            Opcode::FieldStore => "field_store",
            Opcode::ElementGet => "element_get",
            Opcode::ElementAddr => "element_addr",
            Opcode::ElementLoad => "element_load",
            Opcode::ElementSet => "element_set",
            Opcode::ElementStore => "element_store",
            Opcode::Composite => "composite",
            Opcode::VectorSplat => "vector_splat",
            Opcode::VectorExtract => "vector_extract",
            Opcode::VectorInsert => "vector_insert",
            Opcode::VectorShuffle => "vector_shuffle",
            Opcode::VectorSelect => "vector_select",
            Opcode::VectorReduce => "vector_reduce",
            Opcode::VectorCompare => "vector_compare",
            Opcode::VectorConvert => "vector_convert",
            Opcode::TensorLoad => "tensor_load",
            Opcode::TensorSplat => "tensor_splat",
            Opcode::TensorExtract => "tensor_extract",
            Opcode::TensorStore => "tensor_store",
            Opcode::TensorFill => "tensor_fill",
            Opcode::TensorCopy => "tensor_copy",
            Opcode::TensorReshape => "tensor_reshape",
            Opcode::TensorBroadcast => "tensor_broadcast",
            Opcode::TensorTranspose => "tensor_transpose",
            Opcode::TensorCast => "tensor_cast",
            Opcode::TensorView => "tensor_view",
            Opcode::TensorSlice => "tensor_slice",
            Opcode::TensorPad => "tensor_pad",
            Opcode::TensorConcat => "tensor_concat",
            Opcode::TensorReduce => "tensor_reduce",
            Opcode::TensorDot => "tensor_dot",
            Opcode::TensorConvolution => "tensor_convolution",
            Opcode::TensorGather => "tensor_gather",
            Opcode::TensorScatter => "tensor_scatter",
            Opcode::TensorCompare => "tensor_compare",
            Opcode::TensorSelect => "tensor_select",
            Opcode::TensorConvert => "tensor_convert",
            Opcode::New => "new",
            Opcode::NewSlice => "new_slice",
            Opcode::RawAlloc => "raw_alloc",
            Opcode::RawFree => "raw_free",
            Opcode::Dispose => "dispose",
            Opcode::AsyncDispose => "dispose_async",
            Opcode::Pin => "pin",
            Opcode::Unpin => "unpin",
            Opcode::Drop => "drop",
            Opcode::StackAlloc => "stack_alloc",
            Opcode::Assume => "assume",
            Opcode::Intrinsic => "intrinsic",
            Opcode::AtomicLoad => "atomic_load",
            Opcode::AtomicStore => "atomic_store",
            Opcode::AtomicCompareExchange => "atomic_compare_exchange",
            Opcode::AtomicRmw => "atomic_rmw",
            Opcode::AtomicFence => "atomic_fence",
            Opcode::Barrier => "barrier",
            Opcode::Return => "return",
            Opcode::Yield => "yield",
            Opcode::Jump => "jump",
            Opcode::Branch => "branch",
            Opcode::Check => "check",
            Opcode::CompareAndBranch => "compare_and_branch",
            Opcode::CompareAndBranchConst => "compare_and_branch_const",
            Opcode::Switch => "switch",
            Opcode::SwitchTable => "switch_table",
            Opcode::Throw => "throw",
            Opcode::Trap => "trap",
            Opcode::Unreachable => "unreachable",
            Opcode::TailCall => "tail_call",
            Opcode::TailCallSelf => "tail_call_self",
            Opcode::TailCallVirtual => "tail_call_virtual",
            Opcode::TailCallInterface => "tail_call_interface",
            Opcode::TailCallIndirect => "tail_call_indirect",
        }
    }
}
