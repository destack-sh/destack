use destack_mir::{self as mir, LayoutId};

use super::{
    ArgumentRange, CallTarget, CallableObjectLayout, ElementAccess, ElementAccessId, FieldAccessId,
    MoveRange, ScalarLayout, TensorConvolutionId, TensorDotId, TensorGatherId, TensorLayoutId,
    TensorScatterId, TensorWindowId, U32RangeId, ValueLayout, WordLayout,
};
use destack_engine as engine;

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
    /// The memory semantics for the operation.
    pub(crate) semantics: mir::MemorySemantics,
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
    /// The memory semantics for the operation.
    pub(crate) semantics: mir::MemorySemantics,
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
    /// The memory semantics for the operation.
    pub(crate) semantics: mir::MemorySemantics,
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
    /// The memory semantics for the operation.
    pub(crate) semantics: mir::MemorySemantics,
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
    /// The memory semantics for the operation.
    pub(crate) semantics: mir::MemorySemantics,
}

/// Vector splat operation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorSplat {
    /// The destination frame offset.
    pub(crate) dest_offset: u32,
    /// The splatted value word offset.
    pub(crate) value_offset: u32,
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
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
    /// The source element access.
    pub(crate) vector_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The left source element access.
    pub(crate) left_element: ElementAccess,
    /// The right source element access.
    pub(crate) right_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The argument element access.
    pub(crate) argument_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The source element access.
    pub(crate) vector_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The left source element access.
    pub(crate) left_element: ElementAccess,
    /// The right source element access.
    pub(crate) right_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The mask element access.
    pub(crate) mask_element: ElementAccess,
    /// The true branch element access.
    pub(crate) then_element: ElementAccess,
    /// The false branch element access.
    pub(crate) else_element: ElementAccess,
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
    /// The source element access.
    pub(crate) vector_element: ElementAccess,
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
    /// The destination element access.
    pub(crate) dest_element: ElementAccess,
    /// The source element access.
    pub(crate) source_element: ElementAccess,
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
    pub(crate) dest: Option<mir::Value>,
    pub(crate) function: u32,
    pub(crate) target: CallTarget,
    pub(crate) arguments: ArgumentRange,
    pub(crate) moves: MoveRange,
}

/// Function call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallBranch {
    pub(crate) function: u32,
    pub(crate) target: CallTarget,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Virtual method call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallVirtual {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Virtual method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallVirtualBranch {
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Interface method call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterface {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Interface method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterfaceBranch {
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Indirect function call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirect {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) callee_offset: u32,
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    pub(crate) arguments: ArgumentRange,
}

/// Indirect call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirectBranch {
    pub(crate) callee_offset: u32,
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
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
    /// The tensor element access.
    pub(crate) element: ElementAccessId,
}

/// Extract a tensor element from a tensor value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorExtract {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) indices: U32RangeId,
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
    /// The tensor element scalar layout.
    pub(crate) element_layout: ScalarLayout,
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
    /// The tensor element access.
    pub(crate) element: ElementAccessId,
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
    /// The tensor element access.
    pub(crate) element: ElementAccessId,
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
    /// The target element access.
    pub(crate) target_element: ElementAccessId,
    /// The source element access.
    pub(crate) source_element: ElementAccessId,
}

/// Reshape a tensor into a new shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReshape {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) shape: U32RangeId,
    /// The source tensor layout.
    pub(crate) source_layout: TensorLayoutId,
    /// The destination tensor layout.
    pub(crate) dest_layout: TensorLayoutId,
}

/// Broadcast a tensor into a larger shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorBroadcast {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) dimensions: U32RangeId,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Permute tensor dimensions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorTranspose {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) permutation: U32RangeId,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Slice a tensor by offsets, sizes, and strides.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSlice {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) arguments: U32RangeId,
    pub(crate) offsets_count: u16,
    pub(crate) sizes_count: u16,
    pub(crate) strides_count: u16,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Pad a tensor with low, high, and interior padding.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorPad {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) arguments: U32RangeId,
    pub(crate) low_count: u16,
    pub(crate) high_count: u16,
    pub(crate) interior_count: u16,
    pub(crate) value_offset: u32,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Concatenate tensors along a dimension.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConcat {
    pub(crate) dest_offset: u32,
    pub(crate) tensors: U32RangeId,
    pub(crate) tensor_layouts: U32RangeId,
    pub(crate) axis: u32,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Reduce a tensor along axes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReduce {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) initial_offset: u32,
    pub(crate) axes: U32RangeId,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Dot product of two tensors.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorDot {
    pub(crate) dest_offset: u32,
    pub(crate) left_offset: u32,
    pub(crate) right_offset: u32,
    pub(crate) dimensions: TensorDotId,
    pub(crate) left_layout: TensorLayoutId,
    pub(crate) right_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
    pub(crate) element_layout: ScalarLayout,
}

/// Convolution between an input tensor and a kernel tensor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvolution {
    pub(crate) dest_offset: u32,
    pub(crate) input_offset: u32,
    pub(crate) kernel_offset: u32,
    pub(crate) dimensions: TensorConvolutionId,
    pub(crate) window: TensorWindowId,
    pub(crate) feature_group_count: u32,
    pub(crate) batch_group_count: u32,
    pub(crate) input_layout: TensorLayoutId,
    pub(crate) kernel_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
    pub(crate) element_layout: ScalarLayout,
}

/// Gather slices from a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorGather {
    pub(crate) dest_offset: u32,
    pub(crate) source_offset: u32,
    pub(crate) indices_offset: u32,
    pub(crate) dimensions: TensorGatherId,
    pub(crate) slice_sizes: U32RangeId,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) indices_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Scatter updates into a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorScatter {
    pub(crate) dest_offset: u32,
    pub(crate) source_offset: u32,
    pub(crate) indices_offset: u32,
    pub(crate) updates_offset: u32,
    pub(crate) dimensions: TensorScatterId,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) indices_layout: TensorLayoutId,
    pub(crate) updates_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
    pub(crate) element_layout: ScalarLayout,
}

/// Select tensor elements based on a boolean mask.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSelect {
    pub(crate) dest_offset: u32,
    pub(crate) mask_offset: u32,
    pub(crate) then_offset: u32,
    pub(crate) else_offset: u32,
    /// The mask tensor layout.
    pub(crate) mask_layout: TensorLayoutId,
    /// The true branch tensor layout.
    pub(crate) then_layout: TensorLayoutId,
    /// The false branch tensor layout.
    pub(crate) else_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
}

/// Convert a tensor element type.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvert {
    pub(crate) dest_offset: u32,
    pub(crate) tensor_offset: u32,
    pub(crate) source_layout: TensorLayoutId,
    pub(crate) dest_layout: TensorLayoutId,
    pub(crate) source_scalar: ScalarLayout,
    pub(crate) dest_scalar: ScalarLayout,
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
    /// The tensor element access.
    pub(crate) element: ElementAccessId,
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
    pub(crate) function: u32,
    pub(crate) target: CallTarget,
    pub(crate) moves: MoveRange,
}

/// Virtual tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallVirtual {
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Interface tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallInterface {
    pub(crate) receiver_offset: u32,
    pub(crate) table_field: FieldAccessId,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Indirect tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallIndirect {
    pub(crate) callee_offset: u32,
    pub(crate) signature: mir::LocalNodeId<mir::Type>,
    pub(crate) arguments: ArgumentRange,
}
