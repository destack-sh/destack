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
    /// Move one word between frame offsets.
    MoveWord,
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
    // field projection
    // ============================================================================
    /// Compute a fixed-offset address from a frame value.
    AddressFrameValueOffset,
    /// Compute an element address from a frame value.
    AddressFrameValueElement,
    /// Compute a fixed-offset address from a frame pointer.
    AddressFrameOffset,
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
    // element projection
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
    /// Compute an element address from a frame pointer.
    AddressFrameElement,
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
    /// Allocate a zeroed local heap value from a compiled site.
    AllocateHeapSite,
    /// Allocate a zeroed shared heap value from a compiled site.
    AllocateSharedHeapSite,
    /// Allocate a zeroed local slice backing and descriptor.
    AllocateSlice,
    /// Allocate a zeroed shared slice backing and descriptor.
    AllocateSharedSlice,
    /// Allocate local raw memory.
    AllocateRaw,
    /// Allocate shared raw memory.
    AllocateSharedRaw,
    /// Free local raw memory.
    FreeRaw,
    /// Free shared raw memory.
    FreeSharedRaw,
    /// Free local unique heap storage.
    FreeHeap,
    /// Free shared unique heap storage.
    FreeSharedHeap,
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

    // ============================================================================
    // arithmetic and casts
    // ============================================================================
    /// Execute one vector binary kernel.
    VectorBinary,
    /// Add four packed 32-bit integer elements.
    PackedAdd32x4,
    /// Subtract four packed 32-bit integer elements.
    PackedSub32x4,
    /// Multiply four packed 32-bit integer elements.
    PackedMul32x4,
    /// And four packed 32-bit integer elements.
    PackedAnd32x4,
    /// Or four packed 32-bit integer elements.
    PackedOr32x4,
    /// Xor four packed 32-bit integer elements.
    PackedXor32x4,
    /// Shift four packed 32-bit integer elements left.
    PackedShl32x4,
    /// Arithmetically shift four packed 32-bit integer elements right.
    PackedShrI32x4,
    /// Logically shift four packed 32-bit integer elements right.
    PackedShrU32x4,
    /// Add two packed 64-bit integer elements.
    PackedAdd64x2,
    /// Subtract two packed 64-bit integer elements.
    PackedSub64x2,
    /// Multiply two packed 64-bit integer elements.
    PackedMul64x2,
    /// And two packed 64-bit integer elements.
    PackedAnd64x2,
    /// Or two packed 64-bit integer elements.
    PackedOr64x2,
    /// Xor two packed 64-bit integer elements.
    PackedXor64x2,
    /// Shift two packed 64-bit integer elements left.
    PackedShl64x2,
    /// Arithmetically shift two packed 64-bit integer elements right.
    PackedShrI64x2,
    /// Logically shift two packed 64-bit integer elements right.
    PackedShrU64x2,
    /// Add four packed float32 elements.
    PackedAddF32x4,
    /// Subtract four packed float32 elements.
    PackedSubF32x4,
    /// Multiply four packed float32 elements.
    PackedMulF32x4,
    /// Divide four packed float32 elements.
    PackedDivF32x4,
    /// Add two packed float64 elements.
    PackedAddF64x2,
    /// Subtract two packed float64 elements.
    PackedSubF64x2,
    /// Multiply two packed float64 elements.
    PackedMulF64x2,
    /// Divide two packed float64 elements.
    PackedDivF64x2,
    /// Execute one tensor binary kernel.
    TensorBinary,
    /// Execute one contiguous tensor binary kernel.
    TensorContiguousBinary,
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
    /// Add word-stored signed integer values.
    AddWordInt,
    /// Add word-stored unsigned integer values.
    AddWordUint,
    /// Subtract word-stored signed integer values.
    SubWordInt,
    /// Subtract word-stored unsigned integer values.
    SubWordUint,
    /// Multiply word-stored signed integer values.
    MulWordInt,
    /// Multiply word-stored unsigned integer values.
    MulWordUint,
    /// Divide word-stored signed integer values.
    DivWordInt,
    /// Divide word-stored unsigned integer values.
    DivWordUint,
    /// Remainder word-stored signed integer values.
    RemWordInt,
    /// Remainder word-stored unsigned integer values.
    RemWordUint,
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
    /// Arithmetically shift word-stored signed integer values right.
    ShrWordInt,
    /// Logically shift word-stored unsigned integer values right.
    ShrWordUint,
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
    /// Compare word-stored scalar values for equality.
    EqWord,
    /// Compare word-stored scalar values for inequality.
    NeWord,
    /// Compare word-stored signed integers with less than.
    LtWordInt,
    /// Compare word-stored unsigned integers with less than.
    LtWordUint,
    /// Compare word-stored signed integers with less than or equal.
    LeWordInt,
    /// Compare word-stored unsigned integers with less than or equal.
    LeWordUint,
    /// Compare word-stored signed integers with greater than.
    GtWordInt,
    /// Compare word-stored unsigned integers with greater than.
    GtWordUint,
    /// Compare word-stored signed integers with greater than or equal.
    GeWordInt,
    /// Compare word-stored unsigned integers with greater than or equal.
    GeWordUint,
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
    /// Negate a word-stored signed integer value.
    NegWordInt,
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
    /// Execute one vector unary kernel.
    VectorUnary,
    /// Execute one tensor unary kernel.
    TensorUnary,
    /// Negate four packed 32-bit integer elements.
    PackedNegI32x4,
    /// Invert four packed 32-bit integer elements.
    PackedNot32x4,
    /// Negate two packed 64-bit integer elements.
    PackedNegI64x2,
    /// Invert two packed 64-bit integer elements.
    PackedNot64x2,
    /// Negate four packed float32 elements.
    PackedNegF32x4,
    /// Negate two packed float64 elements.
    PackedNegF64x2,
    /// Execute one contiguous tensor unary kernel.
    TensorContiguousUnary,
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
    /// Cast one dense tensor pointer into a tensor view descriptor.
    CastTensorView,

    // ============================================================================
    // calls
    // ============================================================================
    /// Call a known function.
    Call,
    /// Call a known function with an explicit continuation.
    CallBranch,
    /// Call a function pointer.
    CallIndirect,
    /// Call a callable value.
    CallCallable,
    /// Call a function pointer with an explicit continuation.
    CallIndirectBranch,
    /// Call a callable value with an explicit continuation.
    CallCallableBranch,
    /// Call a class method through a local heap receiver.
    CallClassHeap,
    /// Call a class method through a shared heap receiver.
    CallClassSharedHeap,
    /// Call a class method through a local heap receiver with an explicit continuation.
    CallClassHeapBranch,
    /// Call a class method through a shared heap receiver with an explicit continuation.
    CallClassSharedHeapBranch,
    /// Call an interface method through a local heap receiver.
    CallInterfaceHeap,
    /// Call an interface method through a shared heap receiver.
    CallInterfaceSharedHeap,
    /// Call an interface method through a local heap receiver with an explicit continuation.
    CallInterfaceHeapBranch,
    /// Call an interface method through a shared heap receiver with an explicit continuation.
    CallInterfaceSharedHeapBranch,
    /// Tail call a known function.
    TailCall,
    /// Tail call the current function.
    TailCallSelf,
    /// Tail call a function pointer.
    TailCallIndirect,
    /// Tail call a callable value.
    TailCallCallable,
    /// Tail call a class method through a local heap receiver.
    TailCallClassHeap,
    /// Tail call a class method through a shared heap receiver.
    TailCallClassSharedHeap,
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
    /// Branch when word-stored scalar values are equal.
    BranchEqWord,
    /// Branch when word-stored scalar values are not equal.
    BranchNeWord,
    /// Branch when a word-stored signed integer is less than another.
    BranchLtWordInt,
    /// Branch when a word-stored unsigned integer is less than another.
    BranchLtWordUint,
    /// Branch when a word-stored signed integer is less than or equal to another.
    BranchLeWordInt,
    /// Branch when a word-stored unsigned integer is less than or equal to another.
    BranchLeWordUint,
    /// Branch when a word-stored signed integer is greater than another.
    BranchGtWordInt,
    /// Branch when a word-stored unsigned integer is greater than another.
    BranchGtWordUint,
    /// Branch when a word-stored signed integer is greater than or equal to another.
    BranchGeWordInt,
    /// Branch when a word-stored unsigned integer is greater than or equal to another.
    BranchGeWordUint,
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
    /// Switch over integer cases.
    Switch,
    /// Switch over a dense integer case table.
    SwitchTable,
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
    /// Abort execution.
    Abort,
    /// Panic without a runtime payload.
    Panic,
    /// Panic with a runtime payload.
    PanicValue,
    /// Resume an active panic.
    ResumePanic,
    /// Mark unreachable execution.
    Unreachable,

    // ============================================================================
    // explicit memory effects
    // ============================================================================
    /// Record a local heap reference write.
    BarrierWriteHeap,
    /// Record a shared heap reference write.
    BarrierWriteSharedHeap,
    /// Atomically load one scalar value.
    AtomicLoad,
    /// Atomically store one scalar value.
    AtomicStore,
    /// Atomically exchange one scalar value.
    AtomicExchange,
    /// Atomically compare and exchange one scalar value.
    AtomicCompareExchange,
    /// Atomically update one scalar value.
    AtomicReadModifyWrite,
    /// Execute an atomic fence.
    AtomicFence,

    // ============================================================================
    // intrinsics
    // ============================================================================
    /// Execute one intrinsic kernel.
    Intrinsic,

    // ============================================================================
    // vectors
    // ============================================================================
    /// Broadcast a scalar to a vector.
    VectorSplat,
    /// Broadcast one 32-bit scalar to four packed elements.
    PackedSplat32x4,
    /// Broadcast one 64-bit scalar to two packed elements.
    PackedSplat64x2,
    /// Extract one vector element.
    VectorExtract,
    /// Insert one vector element.
    VectorInsert,
    /// Shuffle vector elements.
    VectorShuffle,
    /// Select vector elements.
    VectorSelect,
    /// Reduce vector elements with one kernel.
    VectorReduce,
    /// Convert vector elements with one kernel.
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
    /// Reduce tensor elements with one kernel.
    TensorReduce,
    /// Reduce tensor elements and return source indices.
    TensorIndexReduce,
    /// Compute a tensor dot product.
    TensorDot,
    /// Compute a tensor convolution.
    TensorConvolution,
    /// Gather tensor slices.
    TensorGather,
    /// Scatter tensor slices with one kernel.
    TensorScatter,
    /// Select tensor elements.
    TensorSelect,
    /// Convert tensor elements with one kernel.
    TensorConvert,
    /// Cast tensor storage.
    TensorCast,
    /// Create a tensor view.
    TensorView,
}

// op should fit in 2 bytes
const _: () = assert!(std::mem::size_of::<Op>() <= 2);
