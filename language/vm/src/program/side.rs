use {destack_engine as engine, destack_mir as mir};

use super::{
    ArgumentRange, CallTarget, ElementAccessId, FieldAccessId, MoveRange, TensorConvolutionId,
    TensorDotId, TensorGatherId, TensorScatterId, TensorWindowId, TypeRangeId, U32RangeId,
};

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
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Virtual method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallVirtualBranch {
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Interface method call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterface {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Interface method call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallInterfaceBranch {
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Indirect function call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirect {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) callee: mir::Value,
    pub(crate) arguments: ArgumentRange,
}

/// Indirect call terminator with explicit normal and unwind continuations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallIndirectBranch {
    pub(crate) callee: mir::Value,
    pub(crate) arguments: ArgumentRange,
    pub(crate) normal_state: engine::FrameStateId,
    pub(crate) unwind_state: engine::FrameStateId,
}

/// Load a tensor element from a view.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorLoad {
    pub(crate) dest: mir::Value,
    pub(crate) view: mir::Value,
    pub(crate) indices: ArgumentRange,
    pub(crate) view_type: mir::LocalNodeId<mir::Type>,
    pub(crate) element: Option<ElementAccessId>,
}

/// Extract a tensor element from a tensor value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorExtract {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) indices: ArgumentRange,
    pub(crate) tensor_type: mir::LocalNodeId<mir::Type>,
}

/// Store a tensor element into a view.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorStore {
    pub(crate) view: mir::Value,
    pub(crate) indices: ArgumentRange,
    pub(crate) value: mir::Value,
    pub(crate) view_type: mir::LocalNodeId<mir::Type>,
    pub(crate) element: Option<ElementAccessId>,
}

/// Fill a tensor reference with a scalar value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorFill {
    pub(crate) view: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) view_type: mir::LocalNodeId<mir::Type>,
    pub(crate) element: Option<ElementAccessId>,
}

/// Copy elements between tensor references.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorCopy {
    pub(crate) target: mir::Value,
    pub(crate) source: mir::Value,
    pub(crate) target_type: mir::LocalNodeId<mir::Type>,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) target_element: Option<ElementAccessId>,
    pub(crate) source_element: Option<ElementAccessId>,
}

/// Reshape a tensor into a new shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReshape {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) shape: ArgumentRange,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Broadcast a tensor into a larger shape.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorBroadcast {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) dimensions: U32RangeId,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Permute tensor dimensions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorTranspose {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) permutation: U32RangeId,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Slice a tensor by offsets, sizes, and strides.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSlice {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) arguments: ArgumentRange,
    pub(crate) offsets_count: u16,
    pub(crate) sizes_count: u16,
    pub(crate) strides_count: u16,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Pad a tensor with low, high, and interior padding.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorPad {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
    pub(crate) arguments: ArgumentRange,
    pub(crate) low_count: u16,
    pub(crate) high_count: u16,
    pub(crate) interior_count: u16,
    pub(crate) value: mir::Value,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Concatenate tensors along a dimension.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConcat {
    pub(crate) dest: mir::Value,
    pub(crate) tensors: ArgumentRange,
    pub(crate) tensor_types: TypeRangeId,
    pub(crate) axis: u32,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Reduce a tensor along axes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorReduce {
    pub(crate) dest: mir::Value,
    pub(crate) operator: mir::TensorReduceOperator,
    pub(crate) tensor: mir::Value,
    pub(crate) initial: mir::Value,
    pub(crate) axes: U32RangeId,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Dot product of two tensors.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorDot {
    pub(crate) dest: mir::Value,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
    pub(crate) dimensions: TensorDotId,
    pub(crate) left_type: mir::LocalNodeId<mir::Type>,
    pub(crate) right_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Convolution between an input tensor and a kernel tensor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvolution {
    pub(crate) dest: mir::Value,
    pub(crate) input: mir::Value,
    pub(crate) kernel: mir::Value,
    pub(crate) dimensions: TensorConvolutionId,
    pub(crate) window: TensorWindowId,
    pub(crate) feature_group_count: u32,
    pub(crate) batch_group_count: u32,
    pub(crate) input_type: mir::LocalNodeId<mir::Type>,
    pub(crate) kernel_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Gather slices from a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorGather {
    pub(crate) dest: mir::Value,
    pub(crate) operand: mir::Value,
    pub(crate) indices: mir::Value,
    pub(crate) dimensions: TensorGatherId,
    pub(crate) slice_sizes: U32RangeId,
    pub(crate) operand_type: mir::LocalNodeId<mir::Type>,
    pub(crate) indices_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Scatter updates into a tensor based on indices.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorScatter {
    pub(crate) dest: mir::Value,
    pub(crate) operand: mir::Value,
    pub(crate) indices: mir::Value,
    pub(crate) updates: mir::Value,
    pub(crate) dimensions: TensorScatterId,
    pub(crate) mode: mir::TensorScatterMode,
    pub(crate) operand_type: mir::LocalNodeId<mir::Type>,
    pub(crate) indices_type: mir::LocalNodeId<mir::Type>,
    pub(crate) updates_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Compare two tensors elementwise.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorCompare {
    pub(crate) dest: mir::Value,
    pub(crate) operator: mir::BinaryOperator,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
    pub(crate) left_type: mir::LocalNodeId<mir::Type>,
    pub(crate) right_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Select tensor elements based on a boolean mask.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSelect {
    pub(crate) dest: mir::Value,
    pub(crate) mask: mir::Value,
    pub(crate) then_value: mir::Value,
    pub(crate) else_value: mir::Value,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Convert a tensor element type.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorConvert {
    pub(crate) dest: mir::Value,
    pub(crate) mode: mir::TensorConvertMode,
    pub(crate) tensor: mir::Value,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
}

/// Create a view into a tensor reference.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorView {
    pub(crate) dest: mir::Value,
    pub(crate) view: mir::Value,
    pub(crate) arguments: ArgumentRange,
    pub(crate) offsets_count: u16,
    pub(crate) sizes_count: u16,
    pub(crate) strides_count: u16,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
    pub(crate) element: Option<ElementAccessId>,
}

/// Intrinsic call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Intrinsic {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) intrinsic: mir::Intrinsic,
    pub(crate) arguments: ArgumentRange,
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
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Interface tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallInterface {
    pub(crate) receiver: mir::Value,
    pub(crate) table_field: Option<FieldAccessId>,
    pub(crate) method_index: u32,
    pub(crate) arguments: ArgumentRange,
}

/// Indirect tail call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallIndirect {
    pub(crate) callee: mir::Value,
    pub(crate) arguments: ArgumentRange,
}
