use destack_mir::{
    FloatType, Intrinsic, TensorConvertMode, TensorIndexReduceOperator, TensorIndexTieBreak,
    TensorReduceOperator, TensorScatterMode, VectorConvertMode, VectorReduceOperator,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{AddressSpace, CellLayout, FrameStateId, ScalarFormat};

use super::{
    ArgumentRange, AtomicOrder, AtomicShape, CallTarget, MoveRange, MoveSlot, Projection,
    ProjectionId, SignatureId, TensorAddress, TensorConvolutionId, TensorDotId, TensorGatherId,
    TensorLayoutId, TensorScatterId, TensorWindowId, U32RangeId,
};

const INTRINSIC_ARGUMENT_CAPACITY: usize = 16;

/// Aggregate select operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AggregateSelect {
    /// The destination frame offset.
    pub destination_offset: u32,
    /// The condition cell offset.
    pub condition_offset: u32,
    /// The true source frame offset.
    pub then_offset: u32,
    /// The false source frame offset.
    pub else_offset: u32,
    /// The selected byte length.
    pub byte_len: usize,
}

/// Atomic compare exchange over one scalar value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct AtomicCompareExchange {
    /// The aggregate destination value.
    pub destination: MoveSlot,
    /// The pointer cell offset.
    pub pointer_offset: u32,
    /// The expected value cell offset.
    pub expected_offset: u32,
    /// The replacement value cell offset.
    pub new_value_offset: u32,
    /// The atomic memory shape.
    pub shape: AtomicShape,
    /// The failure ordering.
    pub failure_order: AtomicOrder,
    /// Whether the compare exchange is weak.
    pub is_weak: bool,
}

/// Vector splat operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorSplat {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The splatted value cell offset.
    pub value_offset: u32,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The destination element count.
    pub element_count: u32,
}

/// Vector extract operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorExtract {
    /// The destination cell offset.
    pub dest_offset: u32,
    /// The source vector frame offset.
    pub vector_offset: u32,
    /// The index cell offset.
    pub index_offset: u32,
    /// The source element projection.
    pub vector_element: Projection,
    /// The source element count.
    pub element_count: u32,
}

/// Elementwise vector binary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorBinary {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The left vector frame offset.
    pub left_offset: u32,
    /// The right vector frame offset.
    pub right_offset: u32,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The left source element projection.
    pub left_element: Projection,
    /// The right source element projection.
    pub right_element: Projection,
    /// The element binary kernel.
    pub kernel: ElementBinaryKernel,
    /// The vector element scalar layout.
    pub element_layout: ScalarFormat,
    /// The destination element count.
    pub element_count: u32,
}

/// Elementwise vector unary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorUnary {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The argument vector frame offset.
    pub argument_offset: u32,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The argument element projection.
    pub argument_element: Projection,
    /// The element unary kernel.
    pub kernel: ElementUnaryKernel,
    /// The vector element scalar layout.
    pub element_layout: ScalarFormat,
    /// The destination element count.
    pub element_count: u32,
}

/// Vector insert operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorInsert {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The source vector frame offset.
    pub vector_offset: u32,
    /// The index cell offset.
    pub index_offset: u32,
    /// The inserted value cell offset.
    pub value_offset: u32,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The source element projection.
    pub vector_element: Projection,
    /// The source vector element count.
    pub element_count: u32,
}

/// Vector shuffle operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorShuffle {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The left source frame offset.
    pub left_offset: u32,
    /// The right source frame offset.
    pub right_offset: u32,
    /// The pooled shuffle mask.
    pub mask: U32RangeId,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The left source element projection.
    pub left_element: Projection,
    /// The right source element projection.
    pub right_element: Projection,
    /// The left source element count.
    pub left_count: u32,
    /// The right source element count.
    pub right_count: u32,
}

/// Vector select operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorSelect {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The mask vector frame offset.
    pub mask_offset: u32,
    /// The true branch frame offset.
    pub then_offset: u32,
    /// The false branch frame offset.
    pub else_offset: u32,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The mask element projection.
    pub mask_element: Projection,
    /// The true branch element projection.
    pub then_element: Projection,
    /// The false branch element projection.
    pub else_element: Projection,
    /// The destination element count.
    pub element_count: u32,
}

/// Vector reduction operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorReduce {
    /// The destination cell offset.
    pub dest_offset: u32,
    /// The source vector frame offset.
    pub vector_offset: u32,
    /// The reduction kernel.
    pub kernel: VectorReduceOperator,
    /// The source element projection.
    pub vector_element: Projection,
    /// The vector element scalar layout.
    pub element_layout: ScalarFormat,
    /// The source vector element count.
    pub element_count: u32,
}

/// Vector conversion operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VectorConvert {
    /// The destination frame offset.
    pub dest_offset: u32,
    /// The source vector frame offset.
    pub vector_offset: u32,
    /// The conversion kernel.
    pub mode: VectorConvertMode,
    /// The destination element projection.
    pub dest_element: Projection,
    /// The source element projection.
    pub source_element: Projection,
    /// The destination element scalar layout.
    pub dest_layout: ScalarFormat,
    /// The source element scalar layout.
    pub source_layout: ScalarFormat,
    /// The destination element count.
    pub element_count: u32,
}

/// Function bind operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionBind {
    /// The environment cell layout.
    pub environment: CellLayout,
}

/// Direct function call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Call {
    /// The callee function index.
    pub function: u32,
    /// The resolved call target.
    pub target: CallTarget,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The argument frame moves.
    pub moves: MoveRange,
}

/// Function call terminator with an explicit continuation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallBranch {
    /// The callee function index.
    pub function: u32,
    /// The resolved call target.
    pub target: CallTarget,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The continuation frame state.
    pub target_state: FrameStateId,
}

/// Class method call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallVirtual {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dispatch table field projection.
    pub table_field: ProjectionId,
    /// The dispatch table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}

/// Class method call terminator with an explicit continuation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallVirtualBranch {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dispatch table field projection.
    pub table_field: ProjectionId,
    /// The dispatch table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The continuation frame state.
    pub target_state: FrameStateId,
}

/// Dynamic method call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallDynamic {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dynamic table field projection.
    pub table_field: ProjectionId,
    /// The dynamic table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}

/// Dynamic method call terminator with an explicit continuation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallDynamicBranch {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dynamic table field projection.
    pub table_field: ProjectionId,
    /// The dynamic table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The continuation frame state.
    pub target_state: FrameStateId,
}

/// Indirect function call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IndirectCall {
    /// The callee cell offset.
    pub callee_offset: u32,
    /// The expected function signature.
    pub signature: SignatureId,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}

/// Indirect call terminator with an explicit continuation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IndirectCallBranch {
    /// The callee cell offset.
    pub callee_offset: u32,
    /// The expected function signature.
    pub signature: SignatureId,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The continuation frame state.
    pub target_state: FrameStateId,
}

/// Load a tensor element from a view.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorLoad {
    /// The destination scalar.
    pub dest_offset: u32,
    /// The source tensor view.
    pub view_offset: u32,
    /// The index frame offsets.
    pub indices: U32RangeId,
    /// The tensor view layout.
    pub view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub element: ProjectionId,
    /// The tensor view backing memory.
    pub address: TensorAddress,
}

/// Extract a tensor element from a tensor value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorExtract {
    /// The destination scalar frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The index frame offsets.
    pub indices: U32RangeId,
    /// The source tensor layout.
    pub tensor_layout: TensorLayoutId,
}

/// Elementwise tensor binary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorBinary {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The left tensor frame offset.
    pub left_offset: u32,
    /// The right tensor frame offset.
    pub right_offset: u32,
    /// The left tensor layout.
    pub left_layout: TensorLayoutId,
    /// The right tensor layout.
    pub right_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The element binary kernel.
    pub kernel: ElementBinaryKernel,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
}

/// Elementwise scalar binary kernel.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ElementBinaryKernel {
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
    /// Add over generic float elements.
    AddFloat,
    /// Subtract over generic float elements.
    SubFloat,
    /// Multiply over generic float elements.
    MulFloat,
    /// Divide over generic float elements.
    DivFloat,
    /// Generic float equality.
    EqFloat,
    /// Generic float inequality.
    NeFloat,
    /// Generic float less-than.
    LtFloat,
    /// Generic float less-or-equal.
    LeFloat,
    /// Generic float greater-than.
    GtFloat,
    /// Generic float greater-or-equal.
    GeFloat,
}

/// Contiguous elementwise tensor binary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorContiguousBinary {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The left tensor frame offset.
    pub left_offset: u32,
    /// The right tensor frame offset.
    pub right_offset: u32,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
    /// The contiguous binary kernel selected during lowering.
    pub kernel: ElementBinaryKernel,
}

/// Elementwise tensor unary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorUnary {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub argument_offset: u32,
    /// The argument tensor layout.
    pub argument_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The element unary kernel.
    pub kernel: ElementUnaryKernel,
}

/// Elementwise scalar unary kernel.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ElementUnaryKernel {
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
    /// Negate generic float elements.
    NegFloat,
}

/// Contiguous elementwise tensor unary operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorContiguousUnary {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub argument_offset: u32,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
    /// The contiguous unary kernel selected during lowering.
    pub kernel: ElementUnaryKernel,
}

/// Store a tensor element into a view.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorStore {
    /// The destination tensor view.
    pub view_offset: u32,
    /// The index frame offsets.
    pub indices: U32RangeId,
    /// The stored scalar.
    pub value_offset: u32,
    /// The tensor view layout.
    pub view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub element: ProjectionId,
    /// The tensor view backing memory.
    pub address: TensorAddress,
}

/// Fill a tensor reference with a scalar value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorFill {
    /// The filled tensor view.
    pub view_offset: u32,
    /// The scalar fill value.
    pub value_offset: u32,
    /// The tensor view layout.
    pub view_layout: TensorLayoutId,
    /// The tensor element projection.
    pub element: ProjectionId,
    /// The tensor view backing memory.
    pub address: TensorAddress,
}

/// Copy elements between tensor references.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorCopy {
    /// The target tensor view.
    pub target_offset: u32,
    /// The source tensor view.
    pub source_offset: u32,
    /// The target tensor view layout.
    pub target_layout: TensorLayoutId,
    /// The source tensor view layout.
    pub source_layout: TensorLayoutId,
    /// The target element projection.
    pub target_element: ProjectionId,
    /// The source element projection.
    pub source_element: ProjectionId,
    /// The target tensor backing memory.
    pub target_address: TensorAddress,
    /// The source tensor backing memory.
    pub source_address: TensorAddress,
}

/// Cast a dense pointer into a tensor view descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorViewCast {
    /// The destination tensor view.
    pub dest_offset: u32,
    /// The MIR pointer cell.
    pub pointer_offset: u32,
    /// The tensor view layout.
    pub view_layout: TensorLayoutId,
}

/// Reshape a tensor into a new shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorReshape {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The destination shape values.
    pub shape: U32RangeId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Broadcast a tensor into a larger shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorBroadcast {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The broadcast dimension mapping.
    pub dimensions: U32RangeId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Permute tensor dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorTranspose {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The dimension permutation.
    pub permutation: U32RangeId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Slice a tensor by offsets, sizes, and strides.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorSlice {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The packed offset, size, and stride values.
    pub arguments: U32RangeId,
    /// The number of offset values.
    pub offsets_count: u16,
    /// The number of size values.
    pub sizes_count: u16,
    /// The number of stride values.
    pub strides_count: u16,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Pad a tensor with low, high, and interior padding.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorPad {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The packed low, high, and interior padding values.
    pub arguments: U32RangeId,
    /// The number of low padding values.
    pub low_count: u16,
    /// The number of high padding values.
    pub high_count: u16,
    /// The number of interior padding values.
    pub interior_count: u16,
    /// The padding value cell offset.
    pub value_offset: u32,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Concatenate tensors along a dimension.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorConcat {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offsets.
    pub tensors: U32RangeId,
    /// The source tensor layouts.
    pub tensor_layouts: U32RangeId,
    /// The concatenation axis.
    pub axis: u32,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Reduce a tensor along axes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorReduce {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The initial value cell offset.
    pub initial_offset: u32,
    /// The reduced axis range.
    pub axes: U32RangeId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The reduction kernel.
    pub kernel: TensorReduceOperator,
}

/// Reduce a tensor along one axis and return selected source indices.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorIndexReduce {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The reduced axis.
    pub axis: u32,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The index reduction kernel.
    pub kernel: TensorIndexReduceOperator,
    /// The behavior for equal selected values.
    pub tie_break: TensorIndexTieBreak,
}

/// Dot product of two tensors.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorDot {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The left tensor frame offset.
    pub left_offset: u32,
    /// The right tensor frame offset.
    pub right_offset: u32,
    /// The dot dimension numbers.
    pub dimensions: TensorDotId,
    /// The left tensor layout.
    pub left_layout: TensorLayoutId,
    /// The right tensor layout.
    pub right_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
}

/// Convolution between an input tensor and a kernel tensor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolution {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The input tensor frame offset.
    pub input_offset: u32,
    /// The kernel tensor frame offset.
    pub kernel_offset: u32,
    /// The convolution dimension numbers.
    pub dimensions: TensorConvolutionId,
    /// The convolution window descriptor.
    pub window: TensorWindowId,
    /// The feature group count.
    pub feature_group_count: u32,
    /// The batch group count.
    pub batch_group_count: u32,
    /// The input tensor layout.
    pub input_layout: TensorLayoutId,
    /// The kernel tensor layout.
    pub kernel_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
}

/// Gather slices from a tensor based on indices.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorGather {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub source_offset: u32,
    /// The indices tensor frame offset.
    pub indices_offset: u32,
    /// The gather dimension numbers.
    pub dimensions: TensorGatherId,
    /// The gathered slice sizes.
    pub slice_sizes: U32RangeId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The indices tensor layout.
    pub indices_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Scatter updates into a tensor based on indices.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorScatter {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub source_offset: u32,
    /// The indices tensor frame offset.
    pub indices_offset: u32,
    /// The updates tensor frame offset.
    pub updates_offset: u32,
    /// The scatter dimension numbers.
    pub dimensions: TensorScatterId,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The indices tensor layout.
    pub indices_layout: TensorLayoutId,
    /// The updates tensor layout.
    pub updates_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element scalar layout.
    pub element_layout: ScalarFormat,
    /// The scatter kernel.
    pub mode: TensorScatterMode,
}

/// Select tensor elements based on a boolean mask.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorSelect {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The mask tensor frame offset.
    pub mask_offset: u32,
    /// The true branch tensor frame offset.
    pub then_offset: u32,
    /// The false branch tensor frame offset.
    pub else_offset: u32,
    /// The mask tensor layout.
    pub mask_layout: TensorLayoutId,
    /// The true branch tensor layout.
    pub then_layout: TensorLayoutId,
    /// The false branch tensor layout.
    pub else_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
}

/// Convert a tensor element type.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorConvert {
    /// The destination tensor frame offset.
    pub dest_offset: u32,
    /// The source tensor frame offset.
    pub tensor_offset: u32,
    /// The source tensor layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub dest_layout: TensorLayoutId,
    /// The source tensor scalar layout.
    pub source_scalar: ScalarFormat,
    /// The destination tensor scalar layout.
    pub dest_scalar: ScalarFormat,
    /// The conversion kernel.
    pub mode: TensorConvertMode,
}

/// Create a view into a tensor reference.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorView {
    /// The destination tensor view.
    pub dest_offset: u32,
    /// The source tensor view.
    pub view_offset: u32,
    /// The offset, size, and stride values.
    pub arguments: U32RangeId,
    /// The number of offset values.
    pub offsets_count: u16,
    /// The number of size values.
    pub sizes_count: u16,
    /// The number of stride values.
    pub strides_count: u16,
    /// The source tensor view layout.
    pub source_layout: TensorLayoutId,
    /// The destination tensor view layout.
    pub dest_layout: TensorLayoutId,
    /// The tensor element projection.
    pub element: ProjectionId,
    /// The tensor view backing memory.
    pub address: TensorAddress,
}

/// Intrinsic destination.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub enum IntrinsicDest {
    /// No destination.
    None,
    /// Cell destination frame offset.
    Cell(u32),
    /// Frame-backed destination slot.
    Frame(MoveSlot),
}

/// Intrinsic operand shape carried by VM side records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum IntrinsicOperand {
    /// No value.
    Void,
    /// Boolean value.
    Boolean,
    /// Signed or unsigned integer with width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point value with format.
    Float {
        /// The concrete float format.
        format: FloatType,
    },
    /// Reference value with address space.
    Reference {
        /// The reference address space.
        address_space: AddressSpace,
    },
}

/// Intrinsic call side record.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IntrinsicCall {
    /// The intrinsic kernel.
    pub kernel: Intrinsic,
    /// The destination shape.
    pub dest: IntrinsicDest,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
    /// The lowered operand shape for each argument.
    pub operands: [IntrinsicOperand; INTRINSIC_ARGUMENT_CAPACITY],
    /// The argument count.
    pub operand_count: u8,
}

impl IntrinsicCall {
    /// Return an intrinsic record with static argument operands.
    pub fn new(
        kernel: Intrinsic,
        dest: IntrinsicDest,
        arguments: ArgumentRange,
        operands: &[IntrinsicOperand],
    ) -> Option<Self> {
        if operands.len() > INTRINSIC_ARGUMENT_CAPACITY {
            return None;
        }

        let mut stored = [IntrinsicOperand::Void; INTRINSIC_ARGUMENT_CAPACITY];
        stored[..operands.len()].copy_from_slice(operands);

        Some(Self {
            kernel,
            dest,
            arguments,
            operands: stored,
            operand_count: operands.len() as u8,
        })
    }

    /// Return the lowered argument operands.
    pub fn operands(&self) -> &[IntrinsicOperand] {
        &self.operands[..self.operand_count as usize]
    }
}

/// Tail call to a function.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TailCall {
    /// The callee function index.
    pub function: u32,
    /// The resolved call target.
    pub target: CallTarget,
    /// The argument frame moves.
    pub moves: MoveRange,
}

/// Class tail call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TailCallVirtual {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dispatch table field projection.
    pub table_field: ProjectionId,
    /// The dispatch table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}

/// Dynamic tail call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TailCallDynamic {
    /// The receiver cell offset.
    pub receiver_offset: u32,
    /// The dynamic table field projection.
    pub table_field: ProjectionId,
    /// The dynamic table slot.
    pub slot: u32,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}

/// Indirect tail call.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IndirectTailCall {
    /// The callee cell offset.
    pub callee_offset: u32,
    /// The expected function signature.
    pub signature: SignatureId,
    /// The pooled argument range.
    pub arguments: ArgumentRange,
}
