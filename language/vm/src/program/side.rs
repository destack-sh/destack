use destack_engine as engine;
use destack_mir::{self as mir, LayoutId};

use super::{
    ArgumentRange, CallTarget, CallableObjectLayout, MoveRange, Projection, ProjectionId,
    ScalarLayout, TensorConvolutionId, TensorDotId, TensorGatherId, TensorLayoutId,
    TensorScatterId, TensorWindowId, U32RangeId, ValueLayout, WordLayout,
};

const INTRINSIC_ARGUMENT_CAPACITY: usize = 16;

/// Frame byte select operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameSelect {
    /// The destination frame offset.
    pub(crate) destination_offset: u32,
    /// The condition word offset.
    pub(crate) condition_offset: u32,
    /// The true source frame offset.
    pub(crate) then_offset: u32,
    /// The false source frame offset.
    pub(crate) else_offset: u32,
    /// The selected byte length.
    pub(crate) byte_len: usize,
}

/// Atomic load over one word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicLoad {
    /// The destination word offset.
    pub(crate) dest_offset: u32,
    /// The pointer word offset.
    pub(crate) pointer_offset: u32,
    /// The atomic memory layout.
    pub(crate) layout: WordLayout,
    /// The atomic byte width.
    pub(crate) byte_len: usize,
    /// The memory ordering to apply.
    pub(crate) ordering: mir::MemoryOrdering,
    /// The execution scope for the operation.
    pub(crate) scope: mir::AtomicScope,
    /// The memory scope for the operation.
    pub(crate) memory_scope: mir::MemoryScope,
    /// The memory flags.
    pub(crate) flags: mir::MemoryFlags,
}

/// Atomic store over one word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicStore {
    /// The pointer word offset.
    pub(crate) pointer_offset: u32,
    /// The stored value word offset.
    pub(crate) value_offset: u32,
    /// The atomic memory layout.
    pub(crate) layout: WordLayout,
    /// The atomic byte width.
    pub(crate) byte_len: usize,
    /// The memory ordering to apply.
    pub(crate) ordering: mir::MemoryOrdering,
    /// The execution scope for the operation.
    pub(crate) scope: mir::AtomicScope,
    /// The memory scope for the operation.
    pub(crate) memory_scope: mir::MemoryScope,
    /// The memory flags.
    pub(crate) flags: mir::MemoryFlags,
}

/// Atomic read modify write over one word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicRmw {
    /// The destination word offset.
    pub(crate) dest_offset: u32,
    /// The pointer word offset.
    pub(crate) pointer_offset: u32,
    /// The operator value word offset.
    pub(crate) value_offset: u32,
    /// The atomic memory layout.
    pub(crate) layout: WordLayout,
    /// The atomic byte width.
    pub(crate) byte_len: usize,
    /// The memory ordering to apply.
    pub(crate) ordering: mir::MemoryOrdering,
    /// The execution scope for the operation.
    pub(crate) scope: mir::AtomicScope,
    /// The memory scope for the operation.
    pub(crate) memory_scope: mir::MemoryScope,
    /// The memory flags.
    pub(crate) flags: mir::MemoryFlags,
}

/// Atomic compare exchange over one word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicCompareExchange {
    /// The aggregate destination value.
    pub(crate) dest: mir::Value,
    /// The pointer word offset.
    pub(crate) pointer_offset: u32,
    /// The expected value word offset.
    pub(crate) expected_offset: u32,
    /// The replacement value word offset.
    pub(crate) new_value_offset: u32,
    /// The atomic memory layout.
    pub(crate) layout: WordLayout,
    /// The atomic byte width.
    pub(crate) byte_len: usize,
    /// Whether the compare exchange is weak.
    pub(crate) is_weak: bool,
    /// The memory ordering to apply.
    pub(crate) ordering: mir::MemoryOrdering,
    /// The execution scope for the operation.
    pub(crate) scope: mir::AtomicScope,
    /// The memory scope for the operation.
    pub(crate) memory_scope: mir::MemoryScope,
    /// The memory flags.
    pub(crate) flags: mir::MemoryFlags,
}

/// Atomic fence.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicFence {
    /// The memory ordering to apply.
    pub(crate) ordering: mir::MemoryOrdering,
    /// The execution scope for the operation.
    pub(crate) scope: mir::AtomicScope,
    /// The memory scope for the operation.
    pub(crate) memory_scope: mir::MemoryScope,
    /// The memory flags.
    pub(crate) flags: mir::MemoryFlags,
}

/// Vector splat operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorSplat {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The splatted value word offset.
    pub(crate) value_offset: u32,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The destination element count.
    pub(crate) element_count: u32,
}

/// Vector extract operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorExtract {
    /// The destination word offset.
    pub(crate) dest_offset: u32,
    /// The source vector frame offset.
    pub(crate) vector_offset: u32,
    /// The index word offset.
    pub(crate) index_offset: u32,
    /// The source element projection.
    pub(crate) vector_element: Projection,
    /// The source element count.
    pub(crate) element_count: u32,
}

/// Elementwise vector binary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorBinary {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The left vector frame offset.
    pub(crate) left_offset: u32,
    /// The right vector frame offset.
    pub(crate) right_offset: u32,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The left source element projection.
    pub(crate) left_element: Projection,
    /// The right source element projection.
    pub(crate) right_element: Projection,
    /// The element binary kernel.
    pub(crate) kernel: ElementBinaryKernel,
    /// The vector element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The destination element count.
    pub(crate) element_count: u32,
}

/// Elementwise vector unary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorUnary {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The argument vector frame offset.
    pub(crate) argument_offset: u32,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The argument element projection.
    pub(crate) argument_element: Projection,
    /// The element unary kernel.
    pub(crate) kernel: ElementUnaryKernel,
    /// The vector element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The destination element count.
    pub(crate) element_count: u32,
}

/// Vector insert operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorInsert {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The source vector frame offset.
    pub(crate) vector_offset: u32,
    /// The index word offset.
    pub(crate) index_offset: u32,
    /// The inserted value word offset.
    pub(crate) value_offset: u32,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The source element projection.
    pub(crate) vector_element: Projection,
    /// The source vector element count.
    pub(crate) element_count: u32,
}

/// Vector shuffle operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorShuffle {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The left source frame offset.
    pub(crate) left_offset: u32,
    /// The right source frame offset.
    pub(crate) right_offset: u32,
    /// The pooled shuffle mask.
    pub(crate) mask: U32RangeId,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The left source element projection.
    pub(crate) left_element: Projection,
    /// The right source element projection.
    pub(crate) right_element: Projection,
    /// The left source element count.
    pub(crate) left_count: u32,
    /// The right source element count.
    pub(crate) right_count: u32,
}

/// Vector select operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorSelect {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The mask vector frame offset.
    pub(crate) mask_offset: u32,
    /// The true branch frame offset.
    pub(crate) then_offset: u32,
    /// The false branch frame offset.
    pub(crate) else_offset: u32,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The mask element projection.
    pub(crate) mask_element: Projection,
    /// The true branch element projection.
    pub(crate) then_element: Projection,
    /// The false branch element projection.
    pub(crate) else_element: Projection,
    /// The destination element count.
    pub(crate) element_count: u32,
}

/// Vector reduction operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorReduce {
    /// The destination word offset.
    pub(crate) dest_offset: u32,
    /// The source vector frame offset.
    pub(crate) vector_offset: u32,
    /// The reduction kernel.
    pub(crate) kernel: mir::VectorReduceOperator,
    /// The source element projection.
    pub(crate) vector_element: Projection,
    /// The vector element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The source vector element count.
    pub(crate) element_count: u32,
}

/// Vector conversion operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorConvert {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The source vector frame offset.
    pub(crate) vector_offset: u32,
    /// The conversion kernel.
    pub(crate) mode: mir::VectorConvertMode,
    /// The destination element projection.
    pub(crate) dest_element: Projection,
    /// The source element projection.
    pub(crate) source_element: Projection,
    /// The destination element scalar layout.
    pub(crate) dest_layout: ScalarLayout,
    /// The source element scalar layout.
    pub(crate) source_layout: ScalarLayout,
    /// The destination element count.
    pub(crate) element_count: u32,
}

/// Callable environment representation.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CallableEnvironment {
    /// Environment stored in one VM word.
    Word {
        /// The environment word layout.
        layout: WordLayout,
    },
    /// Environment stored in frame bytes.
    Frame {
        /// The environment heap layout.
        layout: LayoutId,
        /// The environment byte length.
        byte_len: usize,
    },
}

/// Callable bind operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallableBind {
    /// The callable heap layout.
    pub(crate) callable_layout: LayoutId,
    /// The callable object field layout.
    pub(crate) object_layout: CallableObjectLayout,
    /// The environment representation.
    pub(crate) environment: CallableEnvironment,
}

/// Direct function call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Call {
    /// The optional destination value.
    pub(crate) dest: Option<mir::Value>,
    /// The callee function index.
    pub(crate) function: u32,
    /// The resolved call target.
    pub(crate) target: CallTarget,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The argument frame moves.
    pub(crate) moves: MoveRange,
}

/// Function call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallBranch {
    /// The callee function index.
    pub(crate) function: u32,
    /// The resolved call target.
    pub(crate) target: CallTarget,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The normal continuation frame state.
    pub(crate) normal_state: engine::FrameStateId,
    /// The unwind continuation frame state.
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Virtual method call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallVirtual {
    /// The optional destination value.
    pub(crate) dest: Option<mir::Value>,
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The dispatch table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the dispatch table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}

/// Virtual method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallVirtualBranch {
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The dispatch table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the dispatch table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The normal continuation frame state.
    pub(crate) normal_state: engine::FrameStateId,
    /// The unwind continuation frame state.
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Interface method call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterface {
    /// The optional destination value.
    pub(crate) dest: Option<mir::Value>,
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The interface table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the interface table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}

/// Interface method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterfaceBranch {
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The interface table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the interface table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The normal continuation frame state.
    pub(crate) normal_state: engine::FrameStateId,
    /// The unwind continuation frame state.
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Indirect function call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirect {
    /// The optional destination value.
    pub(crate) dest: Option<mir::Value>,
    /// The callee word offset.
    pub(crate) callee_offset: u32,
    /// The expected callable signature.
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}

/// Indirect call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirectBranch {
    /// The callee word offset.
    pub(crate) callee_offset: u32,
    /// The expected callable signature.
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The normal continuation frame state.
    pub(crate) normal_state: engine::FrameStateId,
    /// The unwind continuation frame state.
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Load a tensor element from a view.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorLoad {
    /// The destination scalar.
    pub(crate) dest_offset: u32,
    /// The source tensor view.
    pub(crate) view_offset: u32,
    /// The index frame offsets.
    pub(crate) indices: U32RangeId,
    /// The tensor view layout.
    pub(crate) view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub(crate) element: ProjectionId,
}

/// Extract a tensor element from a tensor value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorExtract {
    /// The destination scalar frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The index frame offsets.
    pub(crate) indices: U32RangeId,
    /// The source tensor layout.
    pub(crate) tensor_layout: TensorLayoutId,
}

/// Elementwise tensor binary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorBinary {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The left tensor frame offset.
    pub(crate) left_offset: u32,
    /// The right tensor frame offset.
    pub(crate) right_offset: u32,
    /// The left tensor layout.
    pub(crate) left_layout: TensorLayoutId,
    /// The right tensor layout.
    pub(crate) right_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The element binary kernel.
    pub(crate) kernel: ElementBinaryKernel,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
}

/// Elementwise scalar binary kernel.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ElementBinaryKernel {
    /// Boolean and over one-byte boolean elements.
    AndBool,
    /// Boolean or over one-byte boolean elements.
    OrBool,
    /// Boolean xor over one-byte boolean elements.
    XorBool,
    /// Boolean equality over one-byte boolean elements.
    EqBool,
    /// Boolean inequality over one-byte boolean elements.
    NeBool,
    /// Wrapping add over integer elements.
    AddInt,
    /// Wrapping subtract over integer elements.
    SubInt,
    /// Wrapping multiply over integer elements.
    MulInt,
    /// Signed divide over integer elements.
    DivInt,
    /// Unsigned divide over integer elements.
    DivUint,
    /// Signed remainder over integer elements.
    RemInt,
    /// Unsigned remainder over integer elements.
    RemUint,
    /// Bitwise and over integer elements.
    AndInt,
    /// Bitwise or over integer elements.
    OrInt,
    /// Bitwise xor over integer elements.
    XorInt,
    /// Shift integer elements left.
    ShlInt,
    /// Arithmetically shift integer elements right.
    ShrInt,
    /// Logically shift integer elements right.
    ShrUint,
    /// Integer equality.
    EqInt,
    /// Integer inequality.
    NeInt,
    /// Signed integer less-than.
    LtInt,
    /// Unsigned integer less-than.
    LtUint,
    /// Signed integer less-or-equal.
    LeInt,
    /// Unsigned integer less-or-equal.
    LeUint,
    /// Signed integer greater-than.
    GtInt,
    /// Unsigned integer greater-than.
    GtUint,
    /// Signed integer greater-or-equal.
    GeInt,
    /// Unsigned integer greater-or-equal.
    GeUint,
    /// Add over float32 elements.
    AddF32,
    /// Subtract over float32 elements.
    SubF32,
    /// Multiply over float32 elements.
    MulF32,
    /// Divide over float32 elements.
    DivF32,
    /// Add over float64 elements.
    AddF64,
    /// Subtract over float64 elements.
    SubF64,
    /// Multiply over float64 elements.
    MulF64,
    /// Divide over float64 elements.
    DivF64,
    /// Float32 equality.
    EqF32,
    /// Float64 equality.
    EqF64,
    /// Float32 inequality.
    NeF32,
    /// Float64 inequality.
    NeF64,
    /// Float32 less-than.
    LtF32,
    /// Float64 less-than.
    LtF64,
    /// Float32 less-or-equal.
    LeF32,
    /// Float64 less-or-equal.
    LeF64,
    /// Float32 greater-than.
    GtF32,
    /// Float64 greater-than.
    GtF64,
    /// Float32 greater-or-equal.
    GeF32,
    /// Float64 greater-or-equal.
    GeF64,
}

/// Contiguous elementwise tensor binary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorContiguousBinary {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The left tensor frame offset.
    pub(crate) left_offset: u32,
    /// The right tensor frame offset.
    pub(crate) right_offset: u32,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The contiguous binary kernel selected during lowering.
    pub(crate) kernel: ElementBinaryKernel,
}

/// Elementwise tensor unary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorUnary {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) argument_offset: u32,
    /// The argument tensor layout.
    pub(crate) argument_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The element unary kernel.
    pub(crate) kernel: ElementUnaryKernel,
}

/// Elementwise scalar unary kernel.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ElementUnaryKernel {
    /// Boolean not over one-byte boolean elements.
    NotBool,
    /// Wrapping negate over signed integer elements.
    NegInt,
    /// Bitwise not over integer elements.
    NotInt,
    /// Negate float32 elements.
    NegF32,
    /// Negate float64 elements.
    NegF64,
}

/// Contiguous elementwise tensor unary operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorContiguousUnary {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) argument_offset: u32,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The contiguous unary kernel selected during lowering.
    pub(crate) kernel: ElementUnaryKernel,
}

/// Store a tensor element into a view.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorStore {
    /// The destination tensor view.
    pub(crate) view_offset: u32,
    /// The index frame offsets.
    pub(crate) indices: U32RangeId,
    /// The stored scalar.
    pub(crate) value_offset: u32,
    /// The tensor view layout.
    pub(crate) view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub(crate) element: ProjectionId,
}

/// Fill a tensor reference with a scalar value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorFill {
    /// The filled tensor view.
    pub(crate) view_offset: u32,
    /// The scalar fill value.
    pub(crate) value_offset: u32,
    /// The tensor view layout.
    pub(crate) view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub(crate) element: ProjectionId,
}

/// Copy elements between tensor references.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorCopy {
    /// The target tensor view.
    pub(crate) target_offset: u32,
    /// The source tensor view.
    pub(crate) source_offset: u32,
    /// The target tensor view layout.
    pub(crate) target_layout: TensorLayoutId,
    /// The source tensor view layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The target element projection.
    pub(crate) target_element: ProjectionId,
    /// The source element projection.
    pub(crate) source_element: ProjectionId,
}

/// Reshape a tensor into a new shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReshape {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The destination shape values.
    pub(crate) shape: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Broadcast a tensor into a larger shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorBroadcast {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The broadcast dimension mapping.
    pub(crate) dimensions: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Permute tensor dimensions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorTranspose {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The dimension permutation.
    pub(crate) permutation: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Slice a tensor by offsets, sizes, and strides.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSlice {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The packed offset, size, and stride values.
    pub(crate) arguments: U32RangeId,
    /// The number of offset values.
    pub(crate) offsets_count: u16,
    /// The number of size values.
    pub(crate) sizes_count: u16,
    /// The number of stride values.
    pub(crate) strides_count: u16,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Pad a tensor with low, high, and interior padding.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorPad {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The packed low, high, and interior padding values.
    pub(crate) arguments: U32RangeId,
    /// The number of low padding values.
    pub(crate) low_count: u16,
    /// The number of high padding values.
    pub(crate) high_count: u16,
    /// The number of interior padding values.
    pub(crate) interior_count: u16,
    /// The padding value word offset.
    pub(crate) value_offset: u32,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Concatenate tensors along a dimension.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConcat {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offsets.
    pub(crate) tensors: U32RangeId,
    /// The source tensor layouts.
    pub(crate) tensor_layouts: U32RangeId,
    /// The concatenation axis.
    pub(crate) axis: u32,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Reduce a tensor along axes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReduce {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The initial value word offset.
    pub(crate) initial_offset: u32,
    /// The reduced axis range.
    pub(crate) axes: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The reduction kernel.
    pub(crate) kernel: mir::TensorReduceOperator,
}

/// Dot product of two tensors.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorDot {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The left tensor frame offset.
    pub(crate) left_offset: u32,
    /// The right tensor frame offset.
    pub(crate) right_offset: u32,
    /// The dot dimension numbers.
    pub(crate) dimensions: TensorDotId,
    /// The left tensor layout.
    pub(crate) left_layout: TensorLayoutId,
    /// The right tensor layout.
    pub(crate) right_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
}

/// Convolution between an input tensor and a kernel tensor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvolution {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The input tensor frame offset.
    pub(crate) input_offset: u32,
    /// The kernel tensor frame offset.
    pub(crate) kernel_offset: u32,
    /// The convolution dimension numbers.
    pub(crate) dimensions: TensorConvolutionId,
    /// The convolution window descriptor.
    pub(crate) window: TensorWindowId,
    /// The feature group count.
    pub(crate) feature_group_count: u32,
    /// The batch group count.
    pub(crate) batch_group_count: u32,
    /// The input tensor layout.
    pub(crate) input_layout: TensorLayoutId,
    /// The kernel tensor layout.
    pub(crate) kernel_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
}

/// Gather slices from a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorGather {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) source_offset: u32,
    /// The indices tensor frame offset.
    pub(crate) indices_offset: u32,
    /// The gather dimension numbers.
    pub(crate) dimensions: TensorGatherId,
    /// The gathered slice sizes.
    pub(crate) slice_sizes: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The indices tensor layout.
    pub(crate) indices_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Scatter updates into a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorScatter {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) source_offset: u32,
    /// The indices tensor frame offset.
    pub(crate) indices_offset: u32,
    /// The updates tensor frame offset.
    pub(crate) updates_offset: u32,
    /// The scatter dimension numbers.
    pub(crate) dimensions: TensorScatterId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The indices tensor layout.
    pub(crate) indices_layout: TensorLayoutId,
    /// The updates tensor layout.
    pub(crate) updates_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
    /// The scatter kernel.
    pub(crate) mode: mir::TensorScatterMode,
}

/// Select tensor elements based on a boolean mask.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSelect {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The mask tensor frame offset.
    pub(crate) mask_offset: u32,
    /// The true branch tensor frame offset.
    pub(crate) then_offset: u32,
    /// The false branch tensor frame offset.
    pub(crate) else_offset: u32,
    /// The mask tensor layout.
    pub(crate) mask_layout: TensorLayoutId,
    /// The true branch tensor layout.
    pub(crate) then_layout: TensorLayoutId,
    /// The false branch tensor layout.
    pub(crate) else_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Convert a tensor element type.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvert {
    /// The destination tensor frame offset.
    pub(crate) dest_offset: u32,
    /// The source tensor frame offset.
    pub(crate) tensor_offset: u32,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The source tensor scalar layout.
    pub(crate) source_scalar: ScalarLayout,
    /// The destination tensor scalar layout.
    pub(crate) dest_scalar: ScalarLayout,
    /// The conversion kernel.
    pub(crate) mode: mir::TensorConvertMode,
}

/// Create a view into a tensor reference.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorView {
    /// The destination tensor view.
    pub(crate) dest_offset: u32,
    /// The source tensor view.
    pub(crate) view_offset: u32,
    /// The offset, size, and stride values.
    pub(crate) arguments: U32RangeId,
    /// The number of offset values.
    pub(crate) offsets_count: u16,
    /// The number of size values.
    pub(crate) sizes_count: u16,
    /// The number of stride values.
    pub(crate) strides_count: u16,
    /// The source tensor view layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor view layout.
    pub(crate) dest_layout: TensorLayoutId,
    /// The tensor element projection.
    pub(crate) element: ProjectionId,
}

/// Intrinsic destination.
#[derive(Clone, Copy, Debug)]
pub(crate) enum IntrinsicDest {
    /// No destination.
    None,
    /// Word destination frame offset.
    Word(u32),
    /// Frame destination value.
    Frame(mir::Value),
}

/// Intrinsic call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Intrinsic {
    /// The intrinsic kernel.
    pub(crate) kernel: mir::Intrinsic,
    /// The destination shape.
    pub(crate) dest: IntrinsicDest,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
    /// The lowered layout for each argument.
    pub(crate) layouts: [ValueLayout; INTRINSIC_ARGUMENT_CAPACITY],
    /// The argument count.
    pub(crate) layout_count: u8,
}

impl Intrinsic {
    /// Return an intrinsic record with static argument layouts.
    pub(crate) fn new(
        kernel: mir::Intrinsic,
        dest: IntrinsicDest,
        arguments: ArgumentRange,
        layouts: &[ValueLayout],
    ) -> Option<Self> {
        if layouts.len() > INTRINSIC_ARGUMENT_CAPACITY {
            return None;
        }

        let mut stored = [ValueLayout::Void; INTRINSIC_ARGUMENT_CAPACITY];
        stored[..layouts.len()].copy_from_slice(layouts);

        Some(Self {
            kernel,
            dest,
            arguments,
            layouts: stored,
            layout_count: layouts.len() as u8,
        })
    }

    /// Return the lowered argument layouts.
    pub(crate) fn layouts(&self) -> &[ValueLayout] {
        &self.layouts[..self.layout_count as usize]
    }
}

/// Tail call to a function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCall {
    /// The callee function index.
    pub(crate) function: u32,
    /// The resolved call target.
    pub(crate) target: CallTarget,
    /// The argument frame moves.
    pub(crate) moves: MoveRange,
}

/// Virtual tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallVirtual {
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The dispatch table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the dispatch table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}

/// Interface tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallInterface {
    /// The receiver word offset.
    pub(crate) receiver_offset: u32,
    /// The interface table field projection.
    pub(crate) table_field: ProjectionId,
    /// The method index inside the interface table.
    pub(crate) method_index: u32,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}

/// Indirect tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallIndirect {
    /// The callee word offset.
    pub(crate) callee_offset: u32,
    /// The expected callable signature.
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    /// The pooled argument range.
    pub(crate) arguments: ArgumentRange,
}
