/// Operation code for one lowered VM instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opcode {
    // ============================================================================
    // values
    // ============================================================================
    /// Load a constant into a frame value.
    LoadConst,
    /// Copy bytes between frame values.
    CopyFrameBytes,
    /// Load bytes from a computed address into a frame value.
    LoadAddressBytes,
    /// Store bytes from a frame value into a computed address.
    StoreAddressBytes,
    /// Select one of two word values.
    Select,

    // ============================================================================
    // locals, statics, functions
    // ============================================================================
    /// Load a local value.
    LoadLocal,
    /// Store a local value.
    StoreLocal,
    /// Compute a local address.
    AddressLocal,
    /// Compute a static address.
    AddressStatic,
    /// Load a word from a static id.
    LoadStaticId,
    /// Store a word to a static id.
    StoreStaticId,
    /// Materialize a function pointer.
    AddressFunction,
    /// Bind a function pointer to one environment.
    BindCallable,
    /// Load the current callable environment.
    LoadCallableEnvironment,

    // ============================================================================
    // word loads
    // ============================================================================
    /// Load a word from local heap memory.
    LoadHeap,
    /// Load a word from shared heap memory.
    LoadSharedHeap,
    /// Load a word from local raw memory.
    LoadRaw,
    /// Load a word from shared raw memory.
    LoadSharedRaw,
    /// Load a word from stack memory.
    LoadStack,
    /// Load a word from frame memory.
    LoadFrame,
    /// Load a word from static memory.
    LoadStatic,

    // ============================================================================
    // word stores
    // ============================================================================
    /// Store a word to local heap memory.
    StoreHeap,
    /// Store a word to shared heap memory.
    StoreSharedHeap,
    /// Store a word to local raw memory.
    StoreRaw,
    /// Store a word to shared raw memory.
    StoreSharedRaw,
    /// Store a word to stack memory.
    StoreStack,
    /// Store a word to frame memory.
    StoreFrame,
    /// Store a word to static memory.
    StoreStatic,

    // ============================================================================
    // field projection
    // ============================================================================
    /// Project a word field from a frame value.
    ProjectField,
    /// Compute a field address in frame memory.
    AddressFrameField,
    /// Compute a field address in local heap memory.
    AddressHeapField,
    /// Compute a field address in shared heap memory.
    AddressSharedHeapField,
    /// Compute a field address in local raw memory.
    AddressRawField,
    /// Compute a field address in shared raw memory.
    AddressSharedRawField,
    /// Compute a field address in stack memory.
    AddressStackField,
    /// Compute a field address in static memory.
    AddressStaticField,
    /// Load a word field through a frame address.
    LoadFrameField,
    /// Load a word field through a local heap reference.
    LoadHeapField,
    /// Load a word field through a shared heap reference.
    LoadSharedHeapField,
    /// Load a word field through a local raw pointer.
    LoadRawField,
    /// Load a word field through a shared raw pointer.
    LoadSharedRawField,
    /// Load a word field through a stack address.
    LoadStackField,
    /// Load a word field through a static address.
    LoadStaticField,
    /// Store a word field through a frame address.
    StoreFrameField,
    /// Store a word field through a local heap reference.
    StoreHeapField,
    /// Store a word field through a shared heap reference.
    StoreSharedHeapField,
    /// Store a word field through a local raw pointer.
    StoreRawField,
    /// Store a word field through a shared raw pointer.
    StoreSharedRawField,
    /// Store a word field through a stack address.
    StoreStackField,
    /// Store a word field through a static address.
    StoreStaticField,

    // ============================================================================
    // element projection
    // ============================================================================
    /// Project a word element from a frame value.
    ProjectElement,
    /// Compute an element address in frame memory.
    AddressFrameElement,
    /// Compute an element address in local heap memory.
    AddressHeapElement,
    /// Compute an element address in shared heap memory.
    AddressSharedHeapElement,
    /// Compute an element address in local raw memory.
    AddressRawElement,
    /// Compute an element address in shared raw memory.
    AddressSharedRawElement,
    /// Compute an element address in stack memory.
    AddressStackElement,
    /// Compute an element address in static memory.
    AddressStaticElement,
    /// Compute an element address through a slice descriptor.
    AddressSliceElement,
    /// Load a word element through a frame address.
    LoadFrameElement,
    /// Load a word element through a local heap reference.
    LoadHeapElement,
    /// Load a word element through a shared heap reference.
    LoadSharedHeapElement,
    /// Load a word element through a local raw pointer.
    LoadRawElement,
    /// Load a word element through a shared raw pointer.
    LoadSharedRawElement,
    /// Load a word element through a stack address.
    LoadStackElement,
    /// Load a word element through a static address.
    LoadStaticElement,
    /// Store a word element through a frame address.
    StoreFrameElement,
    /// Store a word element through a local heap reference.
    StoreHeapElement,
    /// Store a word element through a shared heap reference.
    StoreSharedHeapElement,
    /// Store a word element through a local raw pointer.
    StoreRawElement,
    /// Store a word element through a shared raw pointer.
    StoreSharedRawElement,
    /// Store a word element through a stack address.
    StoreStackElement,
    /// Store a word element through a static address.
    StoreStaticElement,

    // ============================================================================
    // allocation and lifetime
    // ============================================================================
    /// Allocate a zeroed local heap value.
    AllocateHeap,
    /// Allocate a zeroed shared heap value.
    AllocateSharedHeap,
    /// Allocate a zeroed slice backing and descriptor.
    AllocateSlice,
    /// Allocate local raw memory.
    AllocateRaw,
    /// Free local raw memory.
    FreeRaw,
    /// Allocate stack memory.
    AllocateStack,
    /// Run a synchronous disposer.
    Dispose,
    /// Run an asynchronous disposer.
    AsyncDispose,
    /// Pin one value.
    Pin,
    /// Unpin one value.
    Unpin,
    /// Drop one value.
    Drop,

    // ============================================================================
    // arithmetic and casts
    // ============================================================================
    /// Execute a wide signed integer binary operation.
    BinaryWideInt,
    /// Execute a wide unsigned integer binary operation.
    BinaryWideUint,
    /// Execute an elementwise binary operation.
    BinaryElementwise,
    /// And boolean values.
    AndBool,
    /// Or boolean values.
    OrBool,
    /// Xor boolean values.
    XorBool,
    /// Add 32-bit integer values.
    Add32,
    /// Add 64-bit integer values.
    Add64,
    /// Subtract 32-bit integer values.
    Sub32,
    /// Subtract 64-bit integer values.
    Sub64,
    /// Multiply 32-bit integer values.
    Mul32,
    /// Multiply 64-bit integer values.
    Mul64,
    /// Divide signed 32-bit integer values.
    DivI32,
    /// Divide unsigned 32-bit integer values.
    DivU32,
    /// Divide signed 64-bit integer values.
    DivI64,
    /// Divide unsigned 64-bit integer values.
    DivU64,
    /// Remainder signed 32-bit integer values.
    RemI32,
    /// Remainder unsigned 32-bit integer values.
    RemU32,
    /// Remainder signed 64-bit integer values.
    RemI64,
    /// Remainder unsigned 64-bit integer values.
    RemU64,
    /// And 32-bit integer values.
    And32,
    /// And 64-bit integer values.
    And64,
    /// Or 32-bit integer values.
    Or32,
    /// Or 64-bit integer values.
    Or64,
    /// Xor 32-bit integer values.
    Xor32,
    /// Xor 64-bit integer values.
    Xor64,
    /// Shift a 32-bit integer left.
    Shl32,
    /// Shift a 64-bit integer left.
    Shl64,
    /// Arithmetically shift a 32-bit integer right.
    ShrI32,
    /// Logically shift a 32-bit integer right.
    ShrU32,
    /// Arithmetically shift a 64-bit integer right.
    ShrI64,
    /// Logically shift a 64-bit integer right.
    ShrU64,
    /// Add float32 values.
    AddF32,
    /// Add float64 values.
    AddF64,
    /// Subtract float32 values.
    SubF32,
    /// Subtract float64 values.
    SubF64,
    /// Multiply float32 values.
    MulF32,
    /// Multiply float64 values.
    MulF64,
    /// Divide float32 values.
    DivF32,
    /// Divide float64 values.
    DivF64,
    /// Remainder float32 values.
    RemF32,
    /// Remainder float64 values.
    RemF64,
    /// Compare 32-bit integers for equality.
    Eq32,
    /// Compare 64-bit integers for equality.
    Eq64,
    /// Compare 32-bit integers for inequality.
    Ne32,
    /// Compare 64-bit integers for inequality.
    Ne64,
    /// Compare signed 32-bit integers with less than.
    LtI32,
    /// Compare unsigned 32-bit integers with less than.
    LtU32,
    /// Compare signed 64-bit integers with less than.
    LtI64,
    /// Compare unsigned 64-bit integers with less than.
    LtU64,
    /// Compare signed 32-bit integers with less than or equal.
    LeI32,
    /// Compare unsigned 32-bit integers with less than or equal.
    LeU32,
    /// Compare signed 64-bit integers with less than or equal.
    LeI64,
    /// Compare unsigned 64-bit integers with less than or equal.
    LeU64,
    /// Compare signed 32-bit integers with greater than.
    GtI32,
    /// Compare unsigned 32-bit integers with greater than.
    GtU32,
    /// Compare signed 64-bit integers with greater than.
    GtI64,
    /// Compare unsigned 64-bit integers with greater than.
    GtU64,
    /// Compare signed 32-bit integers with greater than or equal.
    GeI32,
    /// Compare unsigned 32-bit integers with greater than or equal.
    GeU32,
    /// Compare signed 64-bit integers with greater than or equal.
    GeI64,
    /// Compare unsigned 64-bit integers with greater than or equal.
    GeU64,
    /// Compare float32 values for equality.
    EqF32,
    /// Compare float64 values for equality.
    EqF64,
    /// Compare float32 values for inequality.
    NeF32,
    /// Compare float64 values for inequality.
    NeF64,
    /// Compare float32 values with less than.
    LtF32,
    /// Compare float64 values with less than.
    LtF64,
    /// Compare float32 values with less than or equal.
    LeF32,
    /// Compare float64 values with less than or equal.
    LeF64,
    /// Compare float32 values with greater than.
    GtF32,
    /// Compare float64 values with greater than.
    GtF64,
    /// Compare float32 values with greater than or equal.
    GeF32,
    /// Compare float64 values with greater than or equal.
    GeF64,
    /// Execute a wide integer unary operation.
    UnaryWideInt,
    /// Negate a signed 32-bit integer.
    NegI32,
    /// Negate a signed 64-bit integer.
    NegI64,
    /// Invert a 32-bit integer.
    Not32,
    /// Invert a 64-bit integer.
    Not64,
    /// Negate a float32 value.
    NegF32,
    /// Negate a float64 value.
    NegF64,
    /// Invert a boolean value.
    NotBool,
    /// Execute an elementwise unary operation.
    UnaryElementwise,
    /// Cast one value.
    Cast,

    // ============================================================================
    // calls
    // ============================================================================
    /// Call a known function.
    Call,
    /// Invoke a known function with normal and unwind targets.
    Invoke,
    /// Call a function pointer.
    CallIndirect,
    /// Invoke a function pointer with normal and unwind targets.
    InvokeIndirect,
    /// Call a virtual method.
    CallVirtual,
    /// Invoke a virtual method with normal and unwind targets.
    InvokeVirtual,
    /// Call an interface method.
    CallInterface,
    /// Invoke an interface method with normal and unwind targets.
    InvokeInterface,
    /// Tail call a known function.
    TailCall,
    /// Tail call the current function.
    TailCallSelf,
    /// Tail call a function pointer.
    TailCallIndirect,
    /// Tail call a virtual method.
    TailCallVirtual,
    /// Tail call an interface method.
    TailCallInterface,

    // ============================================================================
    // control flow
    // ============================================================================
    /// Jump to another block.
    Jump,
    /// Branch on one boolean value.
    BranchBool,
    /// Branch when 32-bit integer values are equal.
    BranchEq32,
    /// Branch when 64-bit integer values are equal.
    BranchEq64,
    /// Branch when 32-bit integer values are not equal.
    BranchNe32,
    /// Branch when 64-bit integer values are not equal.
    BranchNe64,
    /// Branch when a signed 32-bit integer is less than another.
    BranchLtI32,
    /// Branch when an unsigned 32-bit integer is less than another.
    BranchLtU32,
    /// Branch when a signed 64-bit integer is less than another.
    BranchLtI64,
    /// Branch when an unsigned 64-bit integer is less than another.
    BranchLtU64,
    /// Branch when a signed 32-bit integer is less than or equal to another.
    BranchLeI32,
    /// Branch when an unsigned 32-bit integer is less than or equal to another.
    BranchLeU32,
    /// Branch when a signed 64-bit integer is less than or equal to another.
    BranchLeI64,
    /// Branch when an unsigned 64-bit integer is less than or equal to another.
    BranchLeU64,
    /// Branch when a signed 32-bit integer is greater than another.
    BranchGtI32,
    /// Branch when an unsigned 32-bit integer is greater than another.
    BranchGtU32,
    /// Branch when a signed 64-bit integer is greater than another.
    BranchGtI64,
    /// Branch when an unsigned 64-bit integer is greater than another.
    BranchGtU64,
    /// Branch when a signed 32-bit integer is greater than or equal to another.
    BranchGeI32,
    /// Branch when an unsigned 32-bit integer is greater than or equal to another.
    BranchGeU32,
    /// Branch when a signed 64-bit integer is greater than or equal to another.
    BranchGeI64,
    /// Branch when an unsigned 64-bit integer is greater than or equal to another.
    BranchGeU64,
    /// Branch when float32 values are equal.
    BranchEqF32,
    /// Branch when float64 values are equal.
    BranchEqF64,
    /// Branch when float32 values are not equal.
    BranchNeF32,
    /// Branch when float64 values are not equal.
    BranchNeF64,
    /// Branch when a float32 value is less than another.
    BranchLtF32,
    /// Branch when a float64 value is less than another.
    BranchLtF64,
    /// Branch when a float32 value is less than or equal to another.
    BranchLeF32,
    /// Branch when a float64 value is less than or equal to another.
    BranchLeF64,
    /// Branch when a float32 value is greater than another.
    BranchGtF32,
    /// Branch when a float64 value is greater than another.
    BranchGtF64,
    /// Branch when a float32 value is greater than or equal to another.
    BranchGeF32,
    /// Branch when a float64 value is greater than or equal to another.
    BranchGeF64,
    /// Switch over 32-bit integers using direct cases.
    Switch32,
    /// Switch over 64-bit integers using direct cases.
    Switch64,
    /// Switch over wide integers using direct cases.
    SwitchWideInt,
    /// Switch over 32-bit integers using a dense table.
    SwitchTable32,
    /// Switch over 64-bit integers using a dense table.
    SwitchTable64,
    /// Switch over wide integers using a dense table.
    SwitchTableWideInt,
    /// Validate one runtime constraint.
    Check,
    /// Record an assumed condition.
    Assume,
    /// Return from the current function.
    Return,
    /// Yield from the current function.
    Yield,
    /// Throw one value.
    Throw,
    /// Trap execution.
    Trap,
    /// Mark unreachable execution.
    Unreachable,

    // ============================================================================
    // explicit memory effects
    // ============================================================================
    /// Record a managed reference write.
    BarrierWrite,
    /// Atomically load one word.
    AtomicLoad,
    /// Atomically store one word.
    AtomicStore,
    /// Atomically compare and exchange one word.
    AtomicCompareExchange,
    /// Atomically update one word.
    AtomicRmw,
    /// Apply an atomic fence.
    AtomicFence,

    // ============================================================================
    // intrinsics
    // ============================================================================
    /// Call one intrinsic operation.
    Intrinsic,

    // ============================================================================
    // vectors
    // ============================================================================
    /// Broadcast a scalar to a vector.
    VectorSplat,
    /// Extract one vector element.
    VectorExtract,
    /// Insert one vector element.
    VectorInsert,
    /// Shuffle vector elements.
    VectorShuffle,
    /// Select vector elements.
    VectorSelect,
    /// Reduce vector elements.
    VectorReduce,
    /// Compare vector elements.
    VectorCompare,
    /// Convert vector elements.
    VectorConvert,

    // ============================================================================
    // tensors
    // ============================================================================
    /// Broadcast a scalar to a tensor.
    TensorSplat,
    /// Load one tensor element from a view.
    TensorLoad,
    /// Extract one tensor element from a tensor value.
    TensorExtract,
    /// Store one tensor element into a view.
    TensorStore,
    /// Fill a tensor view.
    TensorFill,
    /// Copy tensor elements between views.
    TensorCopy,
    /// Reshape a tensor value.
    TensorReshape,
    /// Broadcast a tensor value.
    TensorBroadcast,
    /// Transpose a tensor value.
    TensorTranspose,
    /// Slice a tensor value.
    TensorSlice,
    /// Pad a tensor value.
    TensorPad,
    /// Concatenate tensor values.
    TensorConcat,
    /// Reduce a tensor value.
    TensorReduce,
    /// Compute a tensor dot product.
    TensorDot,
    /// Compute a tensor convolution.
    TensorConvolution,
    /// Gather tensor slices.
    TensorGather,
    /// Scatter tensor slices.
    TensorScatter,
    /// Compare tensor elements.
    TensorCompare,
    /// Select tensor elements.
    TensorSelect,
    /// Convert tensor elements.
    TensorConvert,
    /// Cast tensor storage.
    TensorCast,
    /// Create a tensor view.
    TensorView,
}
