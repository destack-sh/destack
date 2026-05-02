use std::mem;

use {destack_engine as engine, destack_mir as mir};

use crate::ReferenceMeta;

use super::{
    AllocationLayoutId, ArgumentRange, CallTarget, CheckId, ConstValueId, ElementAccessId,
    FieldAccessId, FrameAccessId, MoveRange, PointeeAccessId, PointerClass, SliceElementAccessId,
    SwitchCasesId, SwitchTableId, TensorConvolutionId, TensorDotId, TensorGatherId,
    TensorScatterId, TensorWindowId, TypeRangeId, U32RangeId,
};

/// Untagged instruction payload storage.
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub(crate) struct Payload {
    /// The raw payload words.
    words: [u64; 6],
}

impl Payload {
    /// Store one typed payload.
    #[inline(always)]
    pub(crate) fn new<T: Copy>(payload: T) -> Self {
        debug_assert!(mem::size_of::<T>() <= mem::size_of::<Self>());
        debug_assert!(mem::align_of::<T>() <= mem::align_of::<Self>());

        let mut storage = Self { words: [0; 6] };
        unsafe { (storage.words.as_mut_ptr() as *mut T).write(payload) };

        storage
    }

    /// Borrow one typed payload.
    #[inline(always)]
    pub(crate) fn get_ref<T>(&self) -> &T {
        debug_assert!(mem::size_of::<T>() <= mem::size_of::<Self>());
        debug_assert!(mem::align_of::<T>() <= mem::align_of::<Self>());

        unsafe { &*(self.words.as_ptr() as *const T) }
    }
}

// payload should fit in 48 bytes
const _: () = assert!(std::mem::size_of::<Payload>() <= 48);

/// Load one constant value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadConst {
    pub(crate) dest: mir::Value,
    pub(crate) value: ConstValueId,
}

/// Binary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Binary {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::BinaryOperator,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
}

/// Word binary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BinaryWord {
    pub(crate) dest: u32,
    pub(crate) left: u32,
    pub(crate) right: u32,
}

/// Elementwise binary opcode on vector or tensor values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BinaryElementwise {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::BinaryOperator,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
    pub(crate) result_type: mir::LocalNodeId<mir::Type>,
}

/// Integer binary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BinaryInteger {
    pub(crate) dest: u32,
    pub(crate) left: u32,
    pub(crate) right: u32,
    pub(crate) width: u8,
    pub(crate) is_signed: bool,
}

/// Unary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Unary {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::UnaryOperator,
    pub(crate) arg: mir::Value,
}

/// Word unary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnaryWord {
    pub(crate) dest: u32,
    pub(crate) arg: u32,
}

/// Integer unary opcode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnaryInteger {
    pub(crate) dest: u32,
    pub(crate) arg: u32,
    pub(crate) width: u8,
    pub(crate) is_signed: bool,
}

/// Elementwise unary opcode on vector or tensor values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnaryElementwise {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::UnaryOperator,
    pub(crate) arg: mir::Value,
    pub(crate) result_type: mir::LocalNodeId<mir::Type>,
}

/// Word type cast.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CastWord {
    pub(crate) dest: u32,
    pub(crate) op: mir::CastOperator,
    pub(crate) arg: u32,
    pub(crate) to_type: u32,
}

/// Word integer cast into wide integer bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CastWordToWideInt {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::CastOperator,
    pub(crate) arg: u32,
    pub(crate) source_width: u16,
    pub(crate) source_signed: bool,
    pub(crate) dest_width: u16,
}

/// Wide integer byte cast into one word integer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CastWideIntToWord {
    pub(crate) dest: u32,
    pub(crate) op: mir::CastOperator,
    pub(crate) arg: mir::Value,
    pub(crate) source_width: u16,
    pub(crate) source_signed: bool,
    pub(crate) dest_width: u16,
    pub(crate) dest_signed: bool,
}

/// Wide integer byte cast into wide integer bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CastWideInt {
    pub(crate) dest: mir::Value,
    pub(crate) op: mir::CastOperator,
    pub(crate) arg: mir::Value,
    pub(crate) source_width: u16,
    pub(crate) source_signed: bool,
    pub(crate) dest_width: u16,
}

/// Conditional word select.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SelectWord {
    pub(crate) dest: u32,
    pub(crate) condition: u32,
    pub(crate) then_value: u32,
    pub(crate) else_value: u32,
}

/// Conditional frame value select.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SelectFrame {
    pub(crate) dest: mir::Value,
    pub(crate) condition: u32,
    pub(crate) then_value: mir::Value,
    pub(crate) else_value: mir::Value,
}

/// Function call.
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

/// Load local variable.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LocalGet {
    pub(crate) dest: mir::Value,
    pub(crate) local: u32,
}

/// Get local address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LocalAddr {
    pub(crate) dest: mir::Value,
    pub(crate) local: u32,
    pub(crate) reference: ReferenceMeta,
}

/// Store local variable.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LocalSet {
    pub(crate) local: u32,
    pub(crate) value: mir::Value,
}

/// Get static address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StaticAddr {
    pub(crate) dest: mir::Value,
    pub(crate) global: u32,
    pub(crate) reference: ReferenceMeta,
}

/// Get a function pointer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FunctionAddr {
    pub(crate) dest: mir::Value,
    pub(crate) function: u32,
}

/// Bind one environment to a function value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallableBind {
    pub(crate) dest: mir::Value,
    pub(crate) function: u32,
    pub(crate) environment: mir::Value,
}

/// Load the callable environment pointer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallableEnvironment {
    pub(crate) dest: mir::Value,
}

/// Fused static address and load.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StaticLoad {
    pub(crate) dest: mir::Value,
    pub(crate) global: u32,
}

/// Fused static address and store.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StaticStore {
    pub(crate) global: u32,
    pub(crate) value: mir::Value,
    pub(crate) reference: ReferenceMeta,
}

/// Load from pointer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Load {
    pub(crate) dest: mir::Value,
    pub(crate) pointer: mir::Value,
    pub(crate) access: PointeeAccessId,
}

/// Load one word through a frame value access.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadFrame {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) access: FrameAccessId,
}

/// Load one word through a frame element access.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadFrameElement {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) access: FrameAccessId,
}

/// Store to pointer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Store {
    pub(crate) pointer: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) access: PointeeAccessId,
}

/// Store one word through a frame value access.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StoreFrame {
    pub(crate) base: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) access: FrameAccessId,
}

/// Store one word through a frame element access.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StoreFrameElement {
    pub(crate) base: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) access: FrameAccessId,
}

/// Compute a frame value address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AddressFrame {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) access: FrameAccessId,
}

/// Compute a frame element address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AddressFrameElement {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) access: FrameAccessId,
}

/// Move bytes between frame values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MoveFrame {
    pub(crate) destination: mir::Value,
    pub(crate) destination_access: FrameAccessId,
    pub(crate) source: mir::Value,
    pub(crate) source_access: FrameAccessId,
}

/// Load bytes from an address into a frame value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadFrameBytes {
    pub(crate) destination: mir::Value,
    pub(crate) address: mir::Value,
    pub(crate) access: PointeeAccessId,
}

/// Store bytes from a frame value into an address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StoreFrameBytes {
    pub(crate) address: mir::Value,
    pub(crate) source: mir::Value,
    pub(crate) access: PointeeAccessId,
}

/// Get struct or tuple field address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldAddr {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) index: u32,
    pub(crate) reference: ReferenceMeta,
    pub(crate) field_count: u32,
    pub(crate) field: FieldAccessId,
}

/// Load a field through field address plus load.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldLoad {
    pub(crate) dest: mir::Value,
    pub(crate) base: mir::Value,
    pub(crate) index: u32,
    pub(crate) field_count: u32,
    pub(crate) field: FieldAccessId,
}

/// Store a field through field address plus store.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldStore {
    pub(crate) base: mir::Value,
    pub(crate) index: u32,
    pub(crate) value: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) field_count: u32,
    pub(crate) field: FieldAccessId,
}

/// Get array element address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ElementAddr {
    pub(crate) dest: mir::Value,
    pub(crate) array: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) array_length: u64,
    pub(crate) element: ElementAccessId,
}

/// Get slice element address.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SliceElementAddr {
    pub(crate) dest: mir::Value,
    pub(crate) slice: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) access: SliceElementAccessId,
}

/// Load an element through element address plus load.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ElementLoad {
    pub(crate) dest: mir::Value,
    pub(crate) array: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) array_length: u64,
    pub(crate) element: ElementAccessId,
}

/// Broadcast a scalar to all vector elements.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorSplat {
    pub(crate) dest: mir::Value,
    pub(crate) value: mir::Value,
}

/// Extract an element from a vector.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorExtract {
    pub(crate) dest: mir::Value,
    pub(crate) vector: mir::Value,
    pub(crate) index: mir::Value,
}

/// Insert an element into a vector.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorInsert {
    pub(crate) dest: mir::Value,
    pub(crate) vector: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) value: mir::Value,
}

/// Shuffle vector elements using a constant mask.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorShuffle {
    pub(crate) dest: mir::Value,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
    pub(crate) mask: U32RangeId,
}

/// Select vector elements based on a boolean mask.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorSelect {
    pub(crate) dest: mir::Value,
    pub(crate) mask: mir::Value,
    pub(crate) then_value: mir::Value,
    pub(crate) else_value: mir::Value,
}

/// Reduce a vector to a scalar.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorReduce {
    pub(crate) dest: mir::Value,
    pub(crate) operator: mir::VectorReduceOperator,
    pub(crate) vector: mir::Value,
}

/// Compare two vectors elementwise.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorCompare {
    pub(crate) dest: mir::Value,
    pub(crate) operator: mir::BinaryOperator,
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
}

/// Convert vector element types using an explicit mode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorConvert {
    pub(crate) dest: mir::Value,
    pub(crate) mode: mir::VectorConvertMode,
    pub(crate) vector: mir::Value,
    pub(crate) source_type: mir::LocalNodeId<mir::Type>,
    pub(crate) dest_type: mir::LocalNodeId<mir::Type>,
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

/// Broadcast a scalar to all tensor elements.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorSplat {
    pub(crate) dest: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) tensor_type: mir::LocalNodeId<mir::Type>,
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

/// Refine a tensor type without changing its contents.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TensorCast {
    pub(crate) dest: mir::Value,
    pub(crate) tensor: mir::Value,
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

/// Store an element through element address plus store.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ElementStore {
    pub(crate) array: mir::Value,
    pub(crate) index: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) array_length: u64,
    pub(crate) element: ElementAccessId,
}

/// Allocate a managed heap value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct New {
    pub(crate) dest: mir::Value,
    pub(crate) allocation: AllocationLayoutId,
}

/// Allocate a heap array.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NewSlice {
    pub(crate) dest: mir::Value,
    pub(crate) length: mir::Value,
    pub(crate) pointer_class: PointerClass,
    pub(crate) element: AllocationLayoutId,
    pub(crate) element_alignment: usize,
}

/// Allocate raw memory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawAlloc {
    pub(crate) dest: mir::Value,
    pub(crate) byte_len: usize,
}

/// Free raw memory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawFree {
    pub(crate) pointer: mir::Value,
}

/// Run explicit synchronous cleanup.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dispose;

/// Run explicit asynchronous cleanup.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AsyncDispose;

/// Pin one local heap reference.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pin {
    pub(crate) value: mir::Value,
}

/// Release one local heap pin.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnpinValue {
    pub(crate) value: mir::Value,
}

/// End ownership synchronously.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DropValue {
    pub(crate) value: mir::Value,
}

/// Allocate stack bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StackAlloc {
    pub(crate) dest: mir::Value,
    pub(crate) reference: ReferenceMeta,
    pub(crate) allocation_type: mir::LocalNodeId<mir::Type>,
}

/// Assume a condition is true.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Assume;

/// Intrinsic call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Intrinsic {
    pub(crate) dest: Option<mir::Value>,
    pub(crate) intrinsic: mir::Intrinsic,
    pub(crate) arguments: ArgumentRange,
}

/// Atomic load.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicLoad {
    pub(crate) dest: mir::Value,
    pub(crate) pointer: mir::Value,
    pub(crate) raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
}

/// Atomic store.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicStore {
    pub(crate) pointer: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
}

/// Atomic compare exchange.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicCompareExchange {
    pub(crate) dest: mir::Value,
    pub(crate) pointer: mir::Value,
    pub(crate) expected: mir::Value,
    pub(crate) new_value: mir::Value,
    pub(crate) raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
}

/// Atomic read modify write.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicRmw {
    pub(crate) dest: mir::Value,
    pub(crate) operator: mir::AtomicRmwOperator,
    pub(crate) pointer: mir::Value,
    pub(crate) value: mir::Value,
    pub(crate) raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
}

/// Atomic fence.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicFence;

/// Managed barrier write.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BarrierWrite {
    /// The managed object whose reference range changed.
    pub(crate) object: mir::Value,
    /// The byte offset of the changed reference range.
    pub(crate) offset: mir::Value,
    /// The changed byte length.
    pub(crate) byte_len: mir::Value,
    /// The managed heap class for the object reference.
    pub(crate) pointer_class: PointerClass,
}

/// Return from function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Return {
    pub(crate) value: Option<mir::Value>,
}

/// Yield from a coroutine.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Yield {
    pub(crate) value: mir::Value,
    pub(crate) source: mir::Value,
    pub(crate) frame_state: engine::FrameStateId,
}

/// Unconditional jump.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Jump {
    pub(crate) target: u32,
    pub(crate) moves: MoveRange,
}

/// Conditional branch.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Branch {
    pub(crate) condition: mir::Value,
    pub(crate) then_target: u32,
    pub(crate) then_moves: MoveRange,
    pub(crate) else_target: u32,
    pub(crate) else_moves: MoveRange,
}

/// Semantic check with explicit success and failure edges.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Check {
    pub(crate) constraint: CheckId,
    pub(crate) then_target: u32,
    pub(crate) then_moves: MoveRange,
    pub(crate) else_target: u32,
    pub(crate) else_moves: MoveRange,
}

/// Fused compare and branch.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CompareAndBranch {
    pub(crate) left: mir::Value,
    pub(crate) right: mir::Value,
    pub(crate) then_target: u32,
    pub(crate) then_moves: MoveRange,
    pub(crate) else_target: u32,
    pub(crate) else_moves: MoveRange,
}

/// Switch on integer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Switch {
    pub(crate) value: mir::Value,
    pub(crate) cases: SwitchCasesId,
    pub(crate) default_target: u32,
    pub(crate) default_moves: MoveRange,
}

/// Switch via dense jump table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TableSwitch {
    pub(crate) value: mir::Value,
    pub(crate) table: SwitchTableId,
    pub(crate) default_target: u32,
    pub(crate) default_moves: MoveRange,
}

/// Unrecoverable runtime termination.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Trap {
    pub(crate) kind: mir::TrapKind,
    pub(crate) payload: Option<mir::Value>,
}

/// Throw one managed exception object.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Throw {
    pub(crate) value: mir::Value,
}

/// Unreachable code.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Unreachable;

/// Tail call to a function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCall {
    pub(crate) function: u32,
    pub(crate) target: CallTarget,
    pub(crate) moves: MoveRange,
}

/// Tail call to the current function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TailCallSelf {
    pub(crate) entry: u32,
    pub(crate) arguments: ArgumentRange,
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
