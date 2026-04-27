/// Operation code for one decoded program instruction.
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
    /// The `element_addr_static` opcode.
    ElementAddrStatic,
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
    /// The `element_load_static` opcode.
    ElementLoadStatic,
    /// The `element_load_heap` opcode.
    ElementLoadHeap,
    /// The `element_load_raw` opcode.
    ElementLoadRaw,
    /// The `element_load_stack` opcode.
    ElementLoadStack,
    /// The `element_store` opcode.
    ElementStore,
    /// The `element_store_static` opcode.
    ElementStoreStatic,
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
    /// The `field_addr_static` opcode.
    FieldAddrStatic,
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
    /// The `field_load_static` opcode.
    FieldLoadStatic,
    /// The `field_load_heap` opcode.
    FieldLoadHeap,
    /// The `field_load_raw` opcode.
    FieldLoadRaw,
    /// The `field_load_stack` opcode.
    FieldLoadStack,
    /// The `field_store` opcode.
    FieldStore,
    /// The `field_store_static` opcode.
    FieldStoreStatic,
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
    /// The `static_addr` opcode.
    StaticAddr,
    /// The `static_load` opcode.
    StaticLoad,
    /// The `static_store` opcode.
    StaticStore,
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
    /// The `load_static` opcode.
    LoadStatic,
    /// The `load_heap` opcode.
    LoadHeap,
    /// The `load_shared_heap` opcode.
    LoadSharedHeap,
    /// The `load_raw` opcode.
    LoadRaw,
    /// The `load_shared_raw` opcode.
    LoadSharedRaw,
    /// The `load_stack` opcode.
    LoadStack,
    /// The `load_frame` opcode.
    LoadFrame,
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
    /// The `new` opcode.
    New,
    /// The `new.slice` opcode.
    NewSlice,
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
    /// The `store_static` opcode.
    StoreStatic,
    /// The `store_heap` opcode.
    StoreHeap,
    /// The `store_heap_bytes` opcode.
    StoreHeapBytes,
    /// The `store_shared_heap` opcode.
    StoreSharedHeap,
    /// The `store_shared_heap_bytes` opcode.
    StoreSharedHeapBytes,
    /// The `store_raw` opcode.
    StoreRaw,
    /// The `store_shared_raw` opcode.
    StoreSharedRaw,
    /// The `store_stack` opcode.
    StoreStack,
    /// The `store_frame` opcode.
    StoreFrame,
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
