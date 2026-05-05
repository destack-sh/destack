/// Operation executed by one lowered VM instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    // ============================================================================
    // values
    // ============================================================================
    /// Load a word constant.
    LoadConstWord,
    /// Load a byte constant into a frame value.
    LoadConstBytes,
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
    /// Materialize a function pointer.
    AddressFunction,
    /// Bind a function pointer to one word environment.
    BindCallableWord,
    /// Bind a function pointer to one frame address environment.
    BindCallableAddress,
    /// Load the current callable environment.
    LoadCallableEnvironment,

    // ============================================================================
    // scalar loads
    // ============================================================================
    /// Load an unsigned 8-bit scalar from local heap memory.
    LoadHeapU8,
    /// Load a signed 8-bit scalar from local heap memory.
    LoadHeapI8,
    /// Load an unsigned 16-bit scalar from local heap memory.
    LoadHeapU16,
    /// Load a signed 16-bit scalar from local heap memory.
    LoadHeapI16,
    /// Load an unsigned 32-bit scalar from local heap memory.
    LoadHeapU32,
    /// Load a signed 32-bit scalar from local heap memory.
    LoadHeapI32,
    /// Load a 64-bit scalar from local heap memory.
    LoadHeap64,
    /// Load an unsigned 8-bit scalar from shared heap memory.
    LoadSharedHeapU8,
    /// Load a signed 8-bit scalar from shared heap memory.
    LoadSharedHeapI8,
    /// Load an unsigned 16-bit scalar from shared heap memory.
    LoadSharedHeapU16,
    /// Load a signed 16-bit scalar from shared heap memory.
    LoadSharedHeapI16,
    /// Load an unsigned 32-bit scalar from shared heap memory.
    LoadSharedHeapU32,
    /// Load a signed 32-bit scalar from shared heap memory.
    LoadSharedHeapI32,
    /// Load a 64-bit scalar from shared heap memory.
    LoadSharedHeap64,
    /// Load an unsigned 8-bit scalar from local raw memory.
    LoadRawU8,
    /// Load a signed 8-bit scalar from local raw memory.
    LoadRawI8,
    /// Load an unsigned 16-bit scalar from local raw memory.
    LoadRawU16,
    /// Load a signed 16-bit scalar from local raw memory.
    LoadRawI16,
    /// Load an unsigned 32-bit scalar from local raw memory.
    LoadRawU32,
    /// Load a signed 32-bit scalar from local raw memory.
    LoadRawI32,
    /// Load a 64-bit scalar from local raw memory.
    LoadRaw64,
    /// Load an unsigned 8-bit scalar from shared raw memory.
    LoadSharedRawU8,
    /// Load a signed 8-bit scalar from shared raw memory.
    LoadSharedRawI8,
    /// Load an unsigned 16-bit scalar from shared raw memory.
    LoadSharedRawU16,
    /// Load a signed 16-bit scalar from shared raw memory.
    LoadSharedRawI16,
    /// Load an unsigned 32-bit scalar from shared raw memory.
    LoadSharedRawU32,
    /// Load a signed 32-bit scalar from shared raw memory.
    LoadSharedRawI32,
    /// Load a 64-bit scalar from shared raw memory.
    LoadSharedRaw64,
    /// Load an unsigned 8-bit scalar from stack memory.
    LoadStackU8,
    /// Load a signed 8-bit scalar from stack memory.
    LoadStackI8,
    /// Load an unsigned 16-bit scalar from stack memory.
    LoadStackU16,
    /// Load a signed 16-bit scalar from stack memory.
    LoadStackI16,
    /// Load an unsigned 32-bit scalar from stack memory.
    LoadStackU32,
    /// Load a signed 32-bit scalar from stack memory.
    LoadStackI32,
    /// Load a 64-bit scalar from stack memory.
    LoadStack64,
    /// Load an unsigned 8-bit scalar from frame memory.
    LoadFrameU8,
    /// Load a signed 8-bit scalar from frame memory.
    LoadFrameI8,
    /// Load an unsigned 16-bit scalar from frame memory.
    LoadFrameU16,
    /// Load a signed 16-bit scalar from frame memory.
    LoadFrameI16,
    /// Load an unsigned 32-bit scalar from frame memory.
    LoadFrameU32,
    /// Load a signed 32-bit scalar from frame memory.
    LoadFrameI32,
    /// Load a 64-bit scalar from frame memory.
    LoadFrame64,
    /// Load an unsigned 8-bit scalar from a frame value.
    LoadFrameValueU8,
    /// Load a signed 8-bit scalar from a frame value.
    LoadFrameValueI8,
    /// Load an unsigned 16-bit scalar from a frame value.
    LoadFrameValueU16,
    /// Load a signed 16-bit scalar from a frame value.
    LoadFrameValueI16,
    /// Load an unsigned 32-bit scalar from a frame value.
    LoadFrameValueU32,
    /// Load a signed 32-bit scalar from a frame value.
    LoadFrameValueI32,
    /// Load a 64-bit scalar from a frame value.
    LoadFrameValue64,
    /// Load an unsigned 8-bit scalar from static memory.
    LoadStaticU8,
    /// Load a signed 8-bit scalar from static memory.
    LoadStaticI8,
    /// Load an unsigned 16-bit scalar from static memory.
    LoadStaticU16,
    /// Load a signed 16-bit scalar from static memory.
    LoadStaticI16,
    /// Load an unsigned 32-bit scalar from static memory.
    LoadStaticU32,
    /// Load a signed 32-bit scalar from static memory.
    LoadStaticI32,
    /// Load a 64-bit scalar from static memory.
    LoadStatic64,

    // ============================================================================
    // scalar stores
    // ============================================================================
    /// Store an 8-bit scalar to local heap memory.
    StoreHeap8,
    /// Store a 16-bit scalar to local heap memory.
    StoreHeap16,
    /// Store a 32-bit scalar to local heap memory.
    StoreHeap32,
    /// Store a 64-bit scalar to local heap memory.
    StoreHeap64,
    /// Store an 8-bit scalar to shared heap memory.
    StoreSharedHeap8,
    /// Store a 16-bit scalar to shared heap memory.
    StoreSharedHeap16,
    /// Store a 32-bit scalar to shared heap memory.
    StoreSharedHeap32,
    /// Store a 64-bit scalar to shared heap memory.
    StoreSharedHeap64,
    /// Store an 8-bit scalar to local raw memory.
    StoreRaw8,
    /// Store a 16-bit scalar to local raw memory.
    StoreRaw16,
    /// Store a 32-bit scalar to local raw memory.
    StoreRaw32,
    /// Store a 64-bit scalar to local raw memory.
    StoreRaw64,
    /// Store an 8-bit scalar to shared raw memory.
    StoreSharedRaw8,
    /// Store a 16-bit scalar to shared raw memory.
    StoreSharedRaw16,
    /// Store a 32-bit scalar to shared raw memory.
    StoreSharedRaw32,
    /// Store a 64-bit scalar to shared raw memory.
    StoreSharedRaw64,
    /// Store an 8-bit scalar to stack memory.
    StoreStack8,
    /// Store a 16-bit scalar to stack memory.
    StoreStack16,
    /// Store a 32-bit scalar to stack memory.
    StoreStack32,
    /// Store a 64-bit scalar to stack memory.
    StoreStack64,
    /// Store an 8-bit scalar to frame memory.
    StoreFrame8,
    /// Store a 16-bit scalar to frame memory.
    StoreFrame16,
    /// Store a 32-bit scalar to frame memory.
    StoreFrame32,
    /// Store a 64-bit scalar to frame memory.
    StoreFrame64,
    /// Store an 8-bit scalar to a frame value.
    StoreFrameValue8,
    /// Store a 16-bit scalar to a frame value.
    StoreFrameValue16,
    /// Store a 32-bit scalar to a frame value.
    StoreFrameValue32,
    /// Store a 64-bit scalar to a frame value.
    StoreFrameValue64,
    /// Store an 8-bit scalar to static memory.
    StoreStatic8,
    /// Store a 16-bit scalar to static memory.
    StoreStatic16,
    /// Store a 32-bit scalar to static memory.
    StoreStatic32,
    /// Store a 64-bit scalar to static memory.
    StoreStatic64,

    // ============================================================================
    // field access
    // ============================================================================
    /// Compute a fixed-offset address in frame memory.
    AddressFrameOffset,
    /// Compute a frame element address.
    AddressFrameElement,
    /// Compute a fixed-offset address in local heap memory.
    AddressHeapOffset,
    /// Compute a fixed-offset address in shared heap memory.
    AddressSharedHeapOffset,
    /// Compute a fixed-offset address in local raw memory.
    AddressRawOffset,
    /// Compute a fixed-offset address in shared raw memory.
    AddressSharedRawOffset,
    /// Compute a fixed-offset address in stack memory.
    AddressStackOffset,
    /// Compute a fixed-offset address in static memory.
    AddressStaticOffset,

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
    /// And vector booleans.
    VectorAndBool,
    /// And tensor booleans.
    TensorAndBool,
    /// Or vector booleans.
    VectorOrBool,
    /// Or tensor booleans.
    TensorOrBool,
    /// Xor vector booleans.
    VectorXorBool,
    /// Xor tensor booleans.
    TensorXorBool,
    /// Add vector integers.
    VectorAddInt,
    /// Add tensor integers.
    TensorAddInt,
    /// Subtract vector integers.
    VectorSubInt,
    /// Subtract tensor integers.
    TensorSubInt,
    /// Multiply vector integers.
    VectorMulInt,
    /// Multiply tensor integers.
    TensorMulInt,
    /// Divide vector signed integers.
    VectorDivInt,
    /// Divide tensor signed integers.
    TensorDivInt,
    /// Divide vector unsigned integers.
    VectorDivUint,
    /// Divide tensor unsigned integers.
    TensorDivUint,
    /// Remainder vector signed integers.
    VectorRemInt,
    /// Remainder tensor signed integers.
    TensorRemInt,
    /// Remainder vector unsigned integers.
    VectorRemUint,
    /// Remainder tensor unsigned integers.
    TensorRemUint,
    /// And vector integers.
    VectorAndInt,
    /// And tensor integers.
    TensorAndInt,
    /// Or vector integers.
    VectorOrInt,
    /// Or tensor integers.
    TensorOrInt,
    /// Xor vector integers.
    VectorXorInt,
    /// Xor tensor integers.
    TensorXorInt,
    /// Shift vector integers left.
    VectorShlInt,
    /// Shift tensor integers left.
    TensorShlInt,
    /// Arithmetically shift vector integers right.
    VectorShrInt,
    /// Arithmetically shift tensor integers right.
    TensorShrInt,
    /// Logically shift vector integers right.
    VectorShrUint,
    /// Logically shift tensor integers right.
    TensorShrUint,
    /// Add vector float32 values.
    VectorAddF32,
    /// Add tensor float32 values.
    TensorAddF32,
    /// Add vector float64 values.
    VectorAddF64,
    /// Add tensor float64 values.
    TensorAddF64,
    /// Subtract vector float32 values.
    VectorSubF32,
    /// Subtract tensor float32 values.
    TensorSubF32,
    /// Subtract vector float64 values.
    VectorSubF64,
    /// Subtract tensor float64 values.
    TensorSubF64,
    /// Multiply vector float32 values.
    VectorMulF32,
    /// Multiply tensor float32 values.
    TensorMulF32,
    /// Multiply vector float64 values.
    VectorMulF64,
    /// Multiply tensor float64 values.
    TensorMulF64,
    /// Divide vector float32 values.
    VectorDivF32,
    /// Divide tensor float32 values.
    TensorDivF32,
    /// Divide vector float64 values.
    VectorDivF64,
    /// Divide tensor float64 values.
    TensorDivF64,
    /// Compare vector integers for equality.
    VectorEqInt,
    /// Compare tensor integers for equality.
    TensorEqInt,
    /// Compare vector booleans for equality.
    VectorEqBool,
    /// Compare tensor booleans for equality.
    TensorEqBool,
    /// Compare vector integers for inequality.
    VectorNeInt,
    /// Compare tensor integers for inequality.
    TensorNeInt,
    /// Compare vector booleans for inequality.
    VectorNeBool,
    /// Compare tensor booleans for inequality.
    TensorNeBool,
    /// Compare vector signed integers with less than.
    VectorLtInt,
    /// Compare tensor signed integers with less than.
    TensorLtInt,
    /// Compare vector unsigned integers with less than.
    VectorLtUint,
    /// Compare tensor unsigned integers with less than.
    TensorLtUint,
    /// Compare vector signed integers with less than or equal.
    VectorLeInt,
    /// Compare tensor signed integers with less than or equal.
    TensorLeInt,
    /// Compare vector unsigned integers with less than or equal.
    VectorLeUint,
    /// Compare tensor unsigned integers with less than or equal.
    TensorLeUint,
    /// Compare vector signed integers with greater than.
    VectorGtInt,
    /// Compare tensor signed integers with greater than.
    TensorGtInt,
    /// Compare vector unsigned integers with greater than.
    VectorGtUint,
    /// Compare tensor unsigned integers with greater than.
    TensorGtUint,
    /// Compare vector signed integers with greater than or equal.
    VectorGeInt,
    /// Compare tensor signed integers with greater than or equal.
    TensorGeInt,
    /// Compare vector unsigned integers with greater than or equal.
    VectorGeUint,
    /// Compare tensor unsigned integers with greater than or equal.
    TensorGeUint,
    /// Compare vector float32 values for equality.
    VectorEqF32,
    /// Compare tensor float32 values for equality.
    TensorEqF32,
    /// Compare vector float64 values for equality.
    VectorEqF64,
    /// Compare tensor float64 values for equality.
    TensorEqF64,
    /// Compare vector float32 values for inequality.
    VectorNeF32,
    /// Compare tensor float32 values for inequality.
    TensorNeF32,
    /// Compare vector float64 values for inequality.
    VectorNeF64,
    /// Compare tensor float64 values for inequality.
    TensorNeF64,
    /// Compare vector float32 values with less than.
    VectorLtF32,
    /// Compare tensor float32 values with less than.
    TensorLtF32,
    /// Compare vector float64 values with less than.
    VectorLtF64,
    /// Compare tensor float64 values with less than.
    TensorLtF64,
    /// Compare vector float32 values with less than or equal.
    VectorLeF32,
    /// Compare tensor float32 values with less than or equal.
    TensorLeF32,
    /// Compare vector float64 values with less than or equal.
    VectorLeF64,
    /// Compare tensor float64 values with less than or equal.
    TensorLeF64,
    /// Compare vector float32 values with greater than.
    VectorGtF32,
    /// Compare tensor float32 values with greater than.
    TensorGtF32,
    /// Compare vector float64 values with greater than.
    VectorGtF64,
    /// Compare tensor float64 values with greater than.
    TensorGtF64,
    /// Compare vector float32 values with greater than or equal.
    VectorGeF32,
    /// Compare tensor float32 values with greater than or equal.
    TensorGeF32,
    /// Compare vector float64 values with greater than or equal.
    VectorGeF64,
    /// Compare tensor float64 values with greater than or equal.
    TensorGeF64,
    /// And boolean values.
    AndBool,
    /// Or boolean values.
    OrBool,
    /// Xor boolean values.
    XorBool,
    /// Add 32-bit signed integer values.
    AddI32,
    /// Add 32-bit unsigned integer values.
    AddU32,
    /// Add 64-bit signed integer values.
    AddI64,
    /// Add 64-bit unsigned integer values.
    AddU64,
    /// Subtract 32-bit signed integer values.
    SubI32,
    /// Subtract 32-bit unsigned integer values.
    SubU32,
    /// Subtract 64-bit signed integer values.
    SubI64,
    /// Subtract 64-bit unsigned integer values.
    SubU64,
    /// Multiply 32-bit signed integer values.
    MulI32,
    /// Multiply 32-bit unsigned integer values.
    MulU32,
    /// Multiply 64-bit signed integer values.
    MulI64,
    /// Multiply 64-bit unsigned integer values.
    MulU64,
    /// Divide 32-bit signed integer values.
    DivI32,
    /// Divide 32-bit unsigned integer values.
    DivU32,
    /// Divide 64-bit signed integer values.
    DivI64,
    /// Divide 64-bit unsigned integer values.
    DivU64,
    /// Remainder 32-bit signed integer values.
    RemI32,
    /// Remainder 32-bit unsigned integer values.
    RemU32,
    /// Remainder 64-bit signed integer values.
    RemI64,
    /// Remainder 64-bit unsigned integer values.
    RemU64,
    /// Add signed integer values.
    AddInt,
    /// Add unsigned integer values.
    AddUint,
    /// Subtract signed integer values.
    SubInt,
    /// Subtract unsigned integer values.
    SubUint,
    /// Multiply signed integer values.
    MulInt,
    /// Multiply unsigned integer values.
    MulUint,
    /// Divide signed integer values.
    DivInt,
    /// Divide unsigned integer values.
    DivUint,
    /// Remainder signed integer values.
    RemInt,
    /// Remainder unsigned integer values.
    RemUint,
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
    /// Shift 32-bit integer values left.
    Shl32,
    /// Shift 64-bit integer values left.
    Shl64,
    /// Arithmetically shift 32-bit signed integer values right.
    ShrI32,
    /// Logically shift 32-bit unsigned integer values right.
    ShrU32,
    /// Arithmetically shift 64-bit signed integer values right.
    ShrI64,
    /// Logically shift 64-bit unsigned integer values right.
    ShrU64,
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
    /// And word-sized integer values.
    AndWord,
    /// Or word-sized integer values.
    OrWord,
    /// Xor word-sized integer values.
    XorWord,
    /// Shift word-sized integer values left.
    ShlWord,
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
    /// Compare 32-bit integer values for equality.
    Eq32,
    /// Compare 64-bit integer values for equality.
    Eq64,
    /// Compare 32-bit integer values for inequality.
    Ne32,
    /// Compare 64-bit integer values for inequality.
    Ne64,
    /// Compare 32-bit signed integer values with less than.
    LtI32,
    /// Compare 32-bit unsigned integer values with less than.
    LtU32,
    /// Compare 64-bit signed integer values with less than.
    LtI64,
    /// Compare 64-bit unsigned integer values with less than.
    LtU64,
    /// Compare 32-bit signed integer values with less than or equal.
    LeI32,
    /// Compare 32-bit unsigned integer values with less than or equal.
    LeU32,
    /// Compare 64-bit signed integer values with less than or equal.
    LeI64,
    /// Compare 64-bit unsigned integer values with less than or equal.
    LeU64,
    /// Compare 32-bit signed integer values with greater than.
    GtI32,
    /// Compare 32-bit unsigned integer values with greater than.
    GtU32,
    /// Compare 64-bit signed integer values with greater than.
    GtI64,
    /// Compare 64-bit unsigned integer values with greater than.
    GtU64,
    /// Compare 32-bit signed integer values with greater than or equal.
    GeI32,
    /// Compare 32-bit unsigned integer values with greater than or equal.
    GeU32,
    /// Compare 64-bit signed integer values with greater than or equal.
    GeI64,
    /// Compare 64-bit unsigned integer values with greater than or equal.
    GeU64,
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
    /// Negate a 32-bit signed integer value.
    NegI32,
    /// Negate a 64-bit signed integer value.
    NegI64,
    /// Invert a 32-bit integer value.
    Not32,
    /// Invert a 64-bit integer value.
    Not64,
    /// Negate an integer value.
    NegInt,
    /// Invert a word-sized integer value.
    NotWord,
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
    /// Negate vector integers.
    VectorNegInt,
    /// Negate tensor integers.
    TensorNegInt,
    /// Invert vector integers.
    VectorNotInt,
    /// Invert tensor integers.
    TensorNotInt,
    /// Negate vector float32 values.
    VectorNegF32,
    /// Negate tensor float32 values.
    TensorNegF32,
    /// Negate vector float64 values.
    VectorNegF64,
    /// Negate tensor float64 values.
    TensorNegF64,
    /// Invert vector booleans.
    VectorNotBool,
    /// Invert tensor booleans.
    TensorNotBool,
    /// Reinterpret one word value.
    CastBitcast,
    /// Truncate one integer word.
    CastTruncate,
    /// Zero extend one integer word.
    CastZeroExtend,
    /// Sign extend one integer word.
    CastSignExtend,
    /// Convert one float word to a signed integer word.
    CastFloatToSignedInt,
    /// Convert one float word to an unsigned integer word.
    CastFloatToUnsignedInt,
    /// Saturating convert one float word to a signed integer word.
    CastFloatToSignedIntSaturating,
    /// Saturating convert one float word to an unsigned integer word.
    CastFloatToUnsignedIntSaturating,
    /// Convert one signed integer word to a float32 word.
    CastSignedIntToF32,
    /// Convert one signed integer word to a float64 word.
    CastSignedIntToF64,
    /// Convert one unsigned integer word to a float32 word.
    CastUnsignedIntToF32,
    /// Convert one unsigned integer word to a float64 word.
    CastUnsignedIntToF64,
    /// Truncate one float word.
    CastFloatTruncate,
    /// Extend one float word.
    CastFloatExtend,
    /// Convert one pointer word to an integer word.
    CastPointerToInt,
    /// Convert one integer word to a pointer word.
    CastIntToPointer,
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
    /// Call a callable value.
    CallCallable,
    /// Invoke a function pointer with normal and unwind targets.
    InvokeIndirect,
    /// Invoke a callable value with normal and unwind targets.
    InvokeCallable,
    /// Call a virtual method through a local heap receiver.
    CallVirtualHeap,
    /// Call a virtual method through a shared heap receiver.
    CallVirtualSharedHeap,
    /// Invoke a virtual method through a local heap receiver.
    InvokeVirtualHeap,
    /// Invoke a virtual method through a shared heap receiver.
    InvokeVirtualSharedHeap,
    /// Call an interface method through a local heap receiver.
    CallInterfaceHeap,
    /// Call an interface method through a shared heap receiver.
    CallInterfaceSharedHeap,
    /// Invoke an interface method through a local heap receiver.
    InvokeInterfaceHeap,
    /// Invoke an interface method through a shared heap receiver.
    InvokeInterfaceSharedHeap,
    /// Tail call a known function.
    TailCall,
    /// Tail call the current function.
    TailCallSelf,
    /// Tail call a function pointer.
    TailCallIndirect,
    /// Tail call a callable value.
    TailCallCallable,
    /// Tail call a virtual method through a local heap receiver.
    TailCallVirtualHeap,
    /// Tail call a virtual method through a shared heap receiver.
    TailCallVirtualSharedHeap,
    /// Tail call an interface method through a local heap receiver.
    TailCallInterfaceHeap,
    /// Tail call an interface method through a shared heap receiver.
    TailCallInterfaceSharedHeap,

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
    /// Branch when a 32-bit signed integer is less than another.
    BranchLtI32,
    /// Branch when a 32-bit unsigned integer is less than another.
    BranchLtU32,
    /// Branch when a 64-bit signed integer is less than another.
    BranchLtI64,
    /// Branch when a 64-bit unsigned integer is less than another.
    BranchLtU64,
    /// Branch when a 32-bit signed integer is less than or equal to another.
    BranchLeI32,
    /// Branch when a 32-bit unsigned integer is less than or equal to another.
    BranchLeU32,
    /// Branch when a 64-bit signed integer is less than or equal to another.
    BranchLeI64,
    /// Branch when a 64-bit unsigned integer is less than or equal to another.
    BranchLeU64,
    /// Branch when a 32-bit signed integer is greater than another.
    BranchGtI32,
    /// Branch when a 32-bit unsigned integer is greater than another.
    BranchGtU32,
    /// Branch when a 64-bit signed integer is greater than another.
    BranchGtI64,
    /// Branch when a 64-bit unsigned integer is greater than another.
    BranchGtU64,
    /// Branch when a 32-bit signed integer is greater than or equal to another.
    BranchGeI32,
    /// Branch when a 32-bit unsigned integer is greater than or equal to another.
    BranchGeU32,
    /// Branch when a 64-bit signed integer is greater than or equal to another.
    BranchGeI64,
    /// Branch when a 64-bit unsigned integer is greater than or equal to another.
    BranchGeU64,
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
    /// Switch over signed 32-bit integers using direct cases.
    SwitchI32,
    /// Switch over unsigned 32-bit integers using direct cases.
    SwitchU32,
    /// Switch over signed 64-bit integers using direct cases.
    SwitchI64,
    /// Switch over unsigned 64-bit integers using direct cases.
    SwitchU64,
    /// Switch over wide signed integers using direct cases.
    SwitchWideInt,
    /// Switch over wide unsigned integers using direct cases.
    SwitchWideUint,
    /// Switch over signed 32-bit integers using a dense table.
    SwitchTableI32,
    /// Switch over unsigned 32-bit integers using a dense table.
    SwitchTableU32,
    /// Switch over signed 64-bit integers using a dense table.
    SwitchTableI64,
    /// Switch over unsigned 64-bit integers using a dense table.
    SwitchTableU64,
    /// Switch over wide signed integers using a dense table.
    SwitchTableWideInt,
    /// Switch over wide unsigned integers using a dense table.
    SwitchTableWideUint,
    /// Validate one runtime constraint.
    Check,
    /// Record an assumed condition.
    Assume,
    /// Return one word value from the current function.
    ReturnWord,
    /// Return one frame address from the current function.
    ReturnAddress,
    /// Return without a value.
    ReturnVoid,
    /// Yield one word value from the current function.
    YieldWord,
    /// Yield one frame address from the current function.
    YieldAddress,
    /// Throw one word value.
    ThrowWord,
    /// Throw one frame address.
    ThrowAddress,
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
    /// Atomically exchange one word.
    AtomicExchange,
    /// Atomically add one word.
    AtomicAdd,
    /// Atomically subtract one word.
    AtomicSub,
    /// Atomically and one word.
    AtomicAnd,
    /// Atomically or one word.
    AtomicOr,
    /// Atomically xor one word.
    AtomicXor,
    /// Atomically signed min one word.
    AtomicMin,
    /// Atomically signed max one word.
    AtomicMax,
    /// Atomically unsigned min one word.
    AtomicUmin,
    /// Atomically unsigned max one word.
    AtomicUmax,
    /// Atomically add one float word.
    AtomicFadd,
    /// Atomically min one float word.
    AtomicFmin,
    /// Atomically max one float word.
    AtomicFmax,
    /// Apply an atomic fence.
    AtomicFence,

    // ============================================================================
    // intrinsics
    // ============================================================================
    /// Count leading zero bits.
    IntrinsicLeadingZeroCount,
    /// Count trailing zero bits.
    IntrinsicTrailingZeroCount,
    /// Count set bits.
    IntrinsicPopulationCount,
    /// Reverse byte order.
    IntrinsicByteSwap,
    /// Reverse bit order.
    IntrinsicBitReverse,
    /// Rotate bits left.
    IntrinsicRotateLeft,
    /// Rotate bits right.
    IntrinsicRotateRight,
    /// Add and report overflow.
    IntrinsicAddOverflow,
    /// Subtract and report overflow.
    IntrinsicSubOverflow,
    /// Multiply and report overflow.
    IntrinsicMulOverflow,
    /// Add without overflow checks.
    IntrinsicAddUnchecked,
    /// Subtract without overflow checks.
    IntrinsicSubUnchecked,
    /// Multiply without overflow checks.
    IntrinsicMulUnchecked,
    /// Divide without overflow checks.
    IntrinsicDivUnchecked,
    /// Compute remainder without overflow checks.
    IntrinsicRemUnchecked,
    /// Shift left without range checks.
    IntrinsicShlUnchecked,
    /// Shift right without range checks.
    IntrinsicShrUnchecked,
    /// Add with saturation.
    IntrinsicSatAdd,
    /// Subtract with saturation.
    IntrinsicSatSub,
    /// Copy non-overlapping raw memory.
    IntrinsicMemcpy,
    /// Copy possibly-overlapping raw memory.
    IntrinsicMemmove,
    /// Fill raw memory.
    IntrinsicMemset,
    /// Compare raw memory.
    IntrinsicMemcmp,
    /// Hint that raw memory will be read.
    IntrinsicPrefetchRead,
    /// Hint that raw memory will be written.
    IntrinsicPrefetchWrite,
    /// Reinterpret one word.
    IntrinsicTransmute,
    /// Cast a pointer between address spaces.
    IntrinsicAddressSpaceCast,
    /// Compute the byte distance between raw pointers.
    IntrinsicPointerOffsetFrom,
    /// Compare raw word bits.
    IntrinsicRawEq,
    /// Compute square root.
    IntrinsicSqrt,
    /// Compute absolute value.
    IntrinsicAbs,
    /// Compute fused multiply-add.
    IntrinsicFma,
    /// Copy the sign of one float to another.
    IntrinsicCopySign,
    /// Compute float minimum.
    IntrinsicMin,
    /// Compute float maximum.
    IntrinsicMax,
    /// Compute sine.
    IntrinsicSin,
    /// Compute cosine.
    IntrinsicCos,
    /// Compute tangent.
    IntrinsicTan,
    /// Compute arc sine.
    IntrinsicAsin,
    /// Compute arc cosine.
    IntrinsicAcos,
    /// Compute arc tangent.
    IntrinsicAtan,
    /// Compute two-argument arc tangent.
    IntrinsicAtan2,
    /// Compute natural exponent.
    IntrinsicExp,
    /// Compute base-two exponent.
    IntrinsicExp2,
    /// Compute natural logarithm.
    IntrinsicLog,
    /// Compute base-two logarithm.
    IntrinsicLog2,
    /// Compute base-ten logarithm.
    IntrinsicLog10,
    /// Compute power.
    IntrinsicPow,
    /// Round down.
    IntrinsicFloor,
    /// Round up.
    IntrinsicCeil,
    /// Round toward zero.
    IntrinsicTrunc,
    /// Round to nearest.
    IntrinsicRound,
    /// Trigger a debugger breakpoint.
    IntrinsicBreakpoint,
    /// Load the current return address.
    IntrinsicReturnAddress,
    /// Load the current frame address.
    IntrinsicFrameAddress,
    /// Preserve an expected value.
    IntrinsicExpect,
    /// Preserve a value through an optimization barrier.
    IntrinsicBlackBox,

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
    /// Reduce vector elements with addition.
    VectorReduceAdd,
    /// Reduce vector elements with multiplication.
    VectorReduceMultiply,
    /// Reduce vector elements with minimum.
    VectorReduceMin,
    /// Reduce vector elements with maximum.
    VectorReduceMax,
    /// Reduce vector elements with bitwise and.
    VectorReduceAnd,
    /// Reduce vector elements with bitwise or.
    VectorReduceOr,
    /// Reduce vector elements with bitwise xor.
    VectorReduceXor,
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
    /// Reduce tensor elements with addition.
    TensorReduceAdd,
    /// Reduce tensor elements with multiplication.
    TensorReduceMultiply,
    /// Reduce tensor elements with minimum.
    TensorReduceMin,
    /// Reduce tensor elements with maximum.
    TensorReduceMax,
    /// Reduce tensor elements with bitwise and.
    TensorReduceAnd,
    /// Reduce tensor elements with bitwise or.
    TensorReduceOr,
    /// Reduce tensor elements with bitwise xor.
    TensorReduceXor,
    /// Compute a tensor dot product.
    TensorDot,
    /// Compute a tensor convolution.
    TensorConvolution,
    /// Gather tensor slices.
    TensorGather,
    /// Scatter tensor slices by replacing elements.
    TensorScatterReplace,
    /// Scatter tensor slices with addition.
    TensorScatterAdd,
    /// Scatter tensor slices with multiplication.
    TensorScatterMultiply,
    /// Scatter tensor slices with minimum.
    TensorScatterMin,
    /// Scatter tensor slices with maximum.
    TensorScatterMax,
    /// Scatter tensor slices with bitwise and.
    TensorScatterAnd,
    /// Scatter tensor slices with bitwise or.
    TensorScatterOr,
    /// Scatter tensor slices with bitwise xor.
    TensorScatterXor,
    /// Select tensor elements.
    TensorSelect,
    /// Convert tensor elements exactly.
    TensorConvertExact,
    /// Convert tensor elements with round to nearest even.
    TensorConvertRoundTiesEven,
    /// Convert tensor elements with round toward zero.
    TensorConvertRoundTowardZero,
    /// Convert tensor elements with round toward negative infinity.
    TensorConvertRoundFloor,
    /// Convert tensor elements with round toward positive infinity.
    TensorConvertRoundCeil,
    /// Convert tensor elements with saturation.
    TensorConvertSaturate,
    /// Cast tensor storage.
    TensorCast,
    /// Create a tensor view.
    TensorView,
}

// op should fit in 2 bytes
const _: () = assert!(std::mem::size_of::<Op>() <= 2);
