/// Operation executed by one lowered VM instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    // ============================================================================
    // values
    // ============================================================================
    /// Load a constant into a frame value.
    LoadConst,
    /// Move bytes between frame values.
    MoveFrame,
    /// Load bytes from local heap memory into a frame value.
    LoadHeapBytes,
    /// Load bytes from shared heap memory into a frame value.
    LoadSharedHeapBytes,
    /// Load bytes from local raw memory into a frame value.
    LoadRawBytes,
    /// Load bytes from shared raw memory into a frame value.
    LoadSharedRawBytes,
    /// Load bytes from stack memory into a frame value.
    LoadStackBytes,
    /// Load bytes from frame memory into a frame value.
    LoadFrameBytes,
    /// Load bytes from static memory into a frame value.
    LoadStaticBytes,
    /// Store bytes from a frame value into local heap memory.
    StoreHeapBytes,
    /// Store bytes from a frame value into shared heap memory.
    StoreSharedHeapBytes,
    /// Store bytes from a frame value into local raw memory.
    StoreRawBytes,
    /// Store bytes from a frame value into shared raw memory.
    StoreSharedRawBytes,
    /// Store bytes from a frame value into stack memory.
    StoreStackBytes,
    /// Store bytes from a frame value into frame memory.
    StoreFrameBytes,
    /// Store bytes from a frame value into static memory.
    StoreStaticBytes,
    /// Select one of two word values.
    SelectWord,
    /// Select one of two frame values.
    SelectFrame,

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
    /// Load a word from a frame element.
    LoadFrameElement,
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
    /// Store a word to a frame element.
    StoreFrameElement,
    /// Store a word to static memory.
    StoreStatic,

    // ============================================================================
    // field access
    // ============================================================================
    /// Compute an address in frame memory.
    AddressFrame,
    /// Compute a frame element address.
    AddressFrameElement,
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
    // element access
    // ============================================================================
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
    /// Compute a slice element address in local heap memory.
    AddressHeapSliceElement,
    /// Compute a slice element address in shared heap memory.
    AddressSharedHeapSliceElement,
    /// Compute a slice element address in local raw memory.
    AddressRawSliceElement,
    /// Compute a slice element address in shared raw memory.
    AddressSharedRawSliceElement,
    /// Compute a slice element address in stack memory.
    AddressStackSliceElement,
    /// Compute a slice element address in frame memory.
    AddressFrameSliceElement,
    /// Compute a slice element address in static memory.
    AddressStaticSliceElement,
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
    /// Allocate a zeroed small noscan local heap value.
    AllocateHeapSmallNoscan,
    /// Allocate a zeroed local heap value.
    AllocateHeap,
    /// Allocate a zeroed small noscan shared heap value.
    AllocateSharedHeapSmallNoscan,
    /// Allocate a zeroed shared heap value.
    AllocateSharedHeap,
    /// Allocate a zeroed local slice backing and descriptor.
    AllocateSlice,
    /// Allocate a zeroed shared slice backing and descriptor.
    AllocateSharedSlice,
    /// Allocate local raw memory.
    AllocateRaw,
    /// Free local raw memory.
    FreeRaw,
    /// Free shared raw memory.
    FreeSharedRaw,
    /// Allocate stack memory.
    AllocateStack,
    /// Pin one local heap reference.
    PinHeap,
    /// Pin one shared heap reference.
    PinSharedHeap,
    /// Unpin one local heap reference.
    UnpinHeap,
    /// Unpin one shared heap reference.
    UnpinSharedHeap,
    /// Drop one owned local heap reference.
    DropHeap,
    /// Drop one owned shared heap reference.
    DropSharedHeap,
    /// Drop one owned stack allocation.
    DropStack,
    /// Drop one owned local slice backing allocation.
    DropSlice,
    /// Drop one owned shared slice backing allocation.
    DropSharedSlice,

    // ============================================================================
    // arithmetic and casts
    // ============================================================================
    /// Execute an elementwise binary operation.
    BinaryElementwise,
    /// And boolean values.
    AndBool,
    /// Or boolean values.
    OrBool,
    /// Xor boolean values.
    XorBool,
    /// Add integer values.
    AddInt,
    /// Subtract integer values.
    SubInt,
    /// Multiply integer values.
    MulInt,
    /// Divide signed integer values.
    DivInt,
    /// Divide unsigned integer values.
    DivUint,
    /// Remainder signed integer values.
    RemInt,
    /// Remainder unsigned integer values.
    RemUint,
    /// Add wide integer values.
    AddWideInt,
    /// Subtract wide integer values.
    SubWideInt,
    /// Multiply wide integer values.
    MulWideInt,
    /// Divide wide signed integer values.
    DivWideInt,
    /// Divide wide unsigned integer values.
    DivWideUint,
    /// Remainder wide signed integer values.
    RemWideInt,
    /// Remainder wide unsigned integer values.
    RemWideUint,
    /// And integer values.
    AndInt,
    /// Or integer values.
    OrInt,
    /// Xor integer values.
    XorInt,
    /// Shift integer values left.
    ShlInt,
    /// Arithmetically shift integer values right.
    ShrInt,
    /// Logically shift integer values right.
    ShrUint,
    /// And wide integer values.
    AndWideInt,
    /// Or wide integer values.
    OrWideInt,
    /// Xor wide integer values.
    XorWideInt,
    /// Shift wide integer values left.
    ShlWideInt,
    /// Arithmetically shift wide integer values right.
    ShrWideInt,
    /// Logically shift wide integer values right.
    ShrWideUint,
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
    /// Compare integers for equality.
    EqInt,
    /// Compare integers for inequality.
    NeInt,
    /// Compare signed integers with less than.
    LtInt,
    /// Compare unsigned integers with less than.
    LtUint,
    /// Compare signed integers with less than or equal.
    LeInt,
    /// Compare unsigned integers with less than or equal.
    LeUint,
    /// Compare signed integers with greater than.
    GtInt,
    /// Compare unsigned integers with greater than.
    GtUint,
    /// Compare signed integers with greater than or equal.
    GeInt,
    /// Compare unsigned integers with greater than or equal.
    GeUint,
    /// Compare wide integers for equality.
    EqWideInt,
    /// Compare wide integers for inequality.
    NeWideInt,
    /// Compare wide signed integers with less than.
    LtWideInt,
    /// Compare wide unsigned integers with less than.
    LtWideUint,
    /// Compare wide signed integers with less than or equal.
    LeWideInt,
    /// Compare wide unsigned integers with less than or equal.
    LeWideUint,
    /// Compare wide signed integers with greater than.
    GtWideInt,
    /// Compare wide unsigned integers with greater than.
    GtWideUint,
    /// Compare wide signed integers with greater than or equal.
    GeWideInt,
    /// Compare wide unsigned integers with greater than or equal.
    GeWideUint,
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
    /// Negate an integer value.
    NegInt,
    /// Invert an integer value.
    NotInt,
    /// Negate a wide integer value.
    NegWideInt,
    /// Invert a wide integer value.
    NotWideInt,
    /// Negate a float32 value.
    NegF32,
    /// Negate a float64 value.
    NegF64,
    /// Invert a boolean value.
    NotBool,
    /// Execute an elementwise unary operation.
    UnaryElementwise,
    /// Cast one word value.
    CastWord,
    /// Cast one word integer into wide integer bytes.
    CastWordToWideInt,
    /// Cast wide integer bytes into one word integer.
    CastWideIntToWord,
    /// Cast wide integer bytes into wide integer bytes.
    CastWideInt,

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
    /// Branch when integer values are equal.
    BranchEqInt,
    /// Branch when integer values are not equal.
    BranchNeInt,
    /// Branch when a signed integer is less than another.
    BranchLtInt,
    /// Branch when an unsigned integer is less than another.
    BranchLtUint,
    /// Branch when a signed integer is less than or equal to another.
    BranchLeInt,
    /// Branch when an unsigned integer is less than or equal to another.
    BranchLeUint,
    /// Branch when a signed integer is greater than another.
    BranchGtInt,
    /// Branch when an unsigned integer is greater than another.
    BranchGtUint,
    /// Branch when a signed integer is greater than or equal to another.
    BranchGeInt,
    /// Branch when an unsigned integer is greater than or equal to another.
    BranchGeUint,
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
    /// Return without a value.
    ReturnVoid,
    /// Yield from the current function.
    Yield,
    /// Throw one value.
    Throw,
    /// Abort execution.
    Abort,
    /// Panic with a runtime payload.
    Panic,
    /// Mark unreachable execution.
    Unreachable,

    // ============================================================================
    // explicit memory effects
    // ============================================================================
    /// Record a local heap reference write.
    BarrierWriteHeap,
    /// Record a shared heap reference write.
    BarrierWriteSharedHeap,
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
    /// Convert vector elements exactly.
    VectorConvertExact,
    /// Convert vector elements with round to nearest even.
    VectorConvertRoundTiesEven,
    /// Convert vector elements with round toward zero.
    VectorConvertRoundTowardZero,
    /// Convert vector elements with round toward negative infinity.
    VectorConvertRoundFloor,
    /// Convert vector elements with round toward positive infinity.
    VectorConvertRoundCeil,
    /// Convert vector elements with saturation.
    VectorConvertSaturate,

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

// op should fit in 2 bytes
const _: () = assert!(std::mem::size_of::<Op>() <= 2);
