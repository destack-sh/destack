use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{Function, MoveRange, Op, Transfer};

use super::frame::move_values_within_frame;

/// Return the instruction bounds for one block.
#[inline(always)]
fn block_bounds(function: &Function, block: u32) -> Result<(usize, usize), Error> {
    let block = function
        .blocks
        .get(block as usize)
        .ok_or(Error::invalid_instruction())?;
    let start = block.start as usize;
    let end = start + block.len as usize;

    Ok((start, end))
}

/// Enter one local block in the current frame.
#[inline(always)]
fn enter_block(
    activation: &mut Activation<'_>,
    function: &Function,
    block: u32,
    moves: MoveRange,
) -> Result<(usize, usize), Error> {
    let move_pool = function.move_pool.as_slice();

    // move block parameters before retargeting the frame
    let frame = activation.active_frame_mut();
    move_values_within_frame(frame, moves, move_pool)?;

    // retarget the frame to the destination block
    let frame = activation.active_frame_mut();
    frame.block = block;

    block_bounds(function, block)
}

/// Result from dispatching one block with instruction counting.
pub(crate) struct BlockDispatch {
    /// The control transfer produced by the block.
    pub(crate) transfer: Transfer,
    /// The number of instructions executed.
    pub(crate) executed: u64,
}

/// Dispatch one lowered instruction through caller supplied fallthrough and transfer handlers.
macro_rules! dispatch_instruction {
    ($activation:ident, $function:ident, $pc:ident, $block_pc:expr, $step:ident, $transfer:ident) => {
        let instruction = &$function.code[$pc];

        match instruction.op {
            Op::LoadConstWord => $step!(super::execute_load_const_word($activation, instruction)),
            Op::LoadConstBytes => $step!(super::execute_load_const_bytes($activation, instruction)),
            Op::MoveWord => $step!(super::execute_move_word($activation, instruction)),
            Op::MoveFrame => $step!(super::execute_move_frame($activation, instruction)),
            Op::LoadHeapBytes => {
                $step!(super::execute_load_heap_bytes($activation, instruction))
            }
            Op::LoadSharedHeapBytes => {
                $step!(super::execute_load_shared_heap_bytes(
                    $activation,
                    instruction
                ))
            }
            Op::LoadRawBytes => {
                $step!(super::execute_load_raw_bytes($activation, instruction))
            }
            Op::LoadStackBytes => {
                $step!(super::execute_load_stack_bytes($activation, instruction))
            }
            Op::LoadFrameBytes => {
                $step!(super::execute_load_frame_bytes($activation, instruction))
            }
            Op::LoadStaticBytes => {
                $step!(super::execute_load_static_bytes($activation, instruction))
            }
            Op::StoreHeapBytes => {
                $step!(super::execute_store_heap_bytes($activation, instruction))
            }
            Op::StoreSharedHeapBytes => {
                $step!(super::execute_store_shared_heap_bytes(
                    $activation,
                    instruction
                ))
            }
            Op::StoreRawBytes => {
                $step!(super::execute_store_raw_bytes($activation, instruction))
            }
            Op::StoreStackBytes => {
                $step!(super::execute_store_stack_bytes($activation, instruction))
            }
            Op::StoreFrameBytes => {
                $step!(super::execute_store_frame_bytes($activation, instruction))
            }
            Op::StoreStaticBytes => {
                $step!(super::execute_store_static_bytes($activation, instruction))
            }
            Op::SelectWord => $step!(super::execute_select_word($activation, instruction)),
            Op::SelectFrame => $step!(super::execute_select_frame($activation, instruction)),
            Op::AddressLocal => $step!(super::execute_address_local($activation, instruction)),
            Op::AddressStatic => $step!(super::execute_address_static($activation, instruction)),
            Op::AddressFunction => {
                $step!(super::execute_address_function($activation, instruction))
            }
            Op::BindClosureWord => {
                $step!(super::execute_bind_closure_word($activation, instruction))
            }
            Op::BindClosureAddress => {
                $step!(super::execute_bind_closure_address(
                    $activation,
                    instruction
                ))
            }
            Op::LoadClosureEnvironment => {
                $step!({ super::execute_load_closure_environment($activation, instruction) })
            }
            Op::LoadHeapU8 => $step!(super::execute_load_heap_scalar::<1, false>(
                $activation,
                instruction
            )),
            Op::LoadHeapI8 => $step!(super::execute_load_heap_scalar::<1, true>(
                $activation,
                instruction
            )),
            Op::LoadHeapU16 => $step!(super::execute_load_heap_scalar::<2, false>(
                $activation,
                instruction
            )),
            Op::LoadHeapI16 => $step!(super::execute_load_heap_scalar::<2, true>(
                $activation,
                instruction
            )),
            Op::LoadHeapU32 => $step!(super::execute_load_heap_scalar::<4, false>(
                $activation,
                instruction
            )),
            Op::LoadHeapI32 => $step!(super::execute_load_heap_scalar::<4, true>(
                $activation,
                instruction
            )),
            Op::LoadHeap64 => $step!(super::execute_load_heap_scalar::<8, false>(
                $activation,
                instruction
            )),
            Op::LoadSharedHeapU8 => {
                $step!(super::execute_load_shared_heap_scalar::<1, false>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeapI8 => {
                $step!(super::execute_load_shared_heap_scalar::<1, true>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeapU16 => {
                $step!(super::execute_load_shared_heap_scalar::<2, false>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeapI16 => {
                $step!(super::execute_load_shared_heap_scalar::<2, true>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeapU32 => {
                $step!(super::execute_load_shared_heap_scalar::<4, false>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeapI32 => {
                $step!(super::execute_load_shared_heap_scalar::<4, true>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadSharedHeap64 => {
                $step!(super::execute_load_shared_heap_scalar::<8, false>(
                    $activation,
                    instruction
                ))
            }
            Op::LoadRawU8 => $step!(super::execute_load_raw_scalar::<1, false>(
                $activation,
                instruction
            )),
            Op::LoadRawI8 => $step!(super::execute_load_raw_scalar::<1, true>(
                $activation,
                instruction
            )),
            Op::LoadRawU16 => $step!(super::execute_load_raw_scalar::<2, false>(
                $activation,
                instruction
            )),
            Op::LoadRawI16 => $step!(super::execute_load_raw_scalar::<2, true>(
                $activation,
                instruction
            )),
            Op::LoadRawU32 => $step!(super::execute_load_raw_scalar::<4, false>(
                $activation,
                instruction
            )),
            Op::LoadRawI32 => $step!(super::execute_load_raw_scalar::<4, true>(
                $activation,
                instruction
            )),
            Op::LoadRaw64 => $step!(super::execute_load_raw_scalar::<8, false>(
                $activation,
                instruction
            )),
            Op::LoadStackU8 => $step!(super::execute_load_stack_scalar::<1, false>(
                $activation,
                instruction
            )),
            Op::LoadStackI8 => $step!(super::execute_load_stack_scalar::<1, true>(
                $activation,
                instruction
            )),
            Op::LoadStackU16 => $step!(super::execute_load_stack_scalar::<2, false>(
                $activation,
                instruction
            )),
            Op::LoadStackI16 => $step!(super::execute_load_stack_scalar::<2, true>(
                $activation,
                instruction
            )),
            Op::LoadStackU32 => $step!(super::execute_load_stack_scalar::<4, false>(
                $activation,
                instruction
            )),
            Op::LoadStackI32 => $step!(super::execute_load_stack_scalar::<4, true>(
                $activation,
                instruction
            )),
            Op::LoadStack64 => $step!(super::execute_load_stack_scalar::<8, false>(
                $activation,
                instruction
            )),
            Op::LoadFrameU8 => $step!(super::execute_load_frame_scalar::<1, false>(
                $activation,
                instruction
            )),
            Op::LoadFrameI8 => $step!(super::execute_load_frame_scalar::<1, true>(
                $activation,
                instruction
            )),
            Op::LoadFrameU16 => $step!(super::execute_load_frame_scalar::<2, false>(
                $activation,
                instruction
            )),
            Op::LoadFrameI16 => $step!(super::execute_load_frame_scalar::<2, true>(
                $activation,
                instruction
            )),
            Op::LoadFrameU32 => $step!(super::execute_load_frame_scalar::<4, false>(
                $activation,
                instruction
            )),
            Op::LoadFrameI32 => $step!(super::execute_load_frame_scalar::<4, true>(
                $activation,
                instruction
            )),
            Op::LoadFrame64 => $step!(super::execute_load_frame_scalar::<8, false>(
                $activation,
                instruction
            )),
            Op::LoadFrameValueU8 => {
                $step!({
                    super::execute_load_frame_value_scalar::<1, false>($activation, instruction)
                })
            }
            Op::LoadFrameValueI8 => {
                $step!({
                    super::execute_load_frame_value_scalar::<1, true>($activation, instruction)
                })
            }
            Op::LoadFrameValueU16 => {
                $step!({
                    super::execute_load_frame_value_scalar::<2, false>($activation, instruction)
                })
            }
            Op::LoadFrameValueI16 => {
                $step!({
                    super::execute_load_frame_value_scalar::<2, true>($activation, instruction)
                })
            }
            Op::LoadFrameValueU32 => {
                $step!({
                    super::execute_load_frame_value_scalar::<4, false>($activation, instruction)
                })
            }
            Op::LoadFrameValueI32 => {
                $step!({
                    super::execute_load_frame_value_scalar::<4, true>($activation, instruction)
                })
            }
            Op::LoadFrameValue64 => {
                $step!({
                    super::execute_load_frame_value_scalar::<8, false>($activation, instruction)
                })
            }
            Op::LoadStaticU8 => $step!(super::execute_load_static_scalar::<1, false>(
                $activation,
                instruction
            )),
            Op::LoadStaticI8 => $step!(super::execute_load_static_scalar::<1, true>(
                $activation,
                instruction
            )),
            Op::LoadStaticU16 => $step!(super::execute_load_static_scalar::<2, false>(
                $activation,
                instruction
            )),
            Op::LoadStaticI16 => $step!(super::execute_load_static_scalar::<2, true>(
                $activation,
                instruction
            )),
            Op::LoadStaticU32 => $step!(super::execute_load_static_scalar::<4, false>(
                $activation,
                instruction
            )),
            Op::LoadStaticI32 => $step!(super::execute_load_static_scalar::<4, true>(
                $activation,
                instruction
            )),
            Op::LoadStatic64 => $step!(super::execute_load_static_scalar::<8, false>(
                $activation,
                instruction
            )),
            Op::StoreHeap8 => $step!(super::execute_store_heap_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreHeap16 => $step!(super::execute_store_heap_scalar::<2>(
                $activation,
                instruction
            )),
            Op::StoreHeap32 => $step!(super::execute_store_heap_scalar::<4>(
                $activation,
                instruction
            )),
            Op::StoreHeap64 => $step!(super::execute_store_heap_scalar::<8>(
                $activation,
                instruction
            )),
            Op::StoreSharedHeap8 => $step!(super::execute_store_shared_heap_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreSharedHeap16 => {
                $step!(super::execute_store_shared_heap_scalar::<2>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreSharedHeap32 => {
                $step!(super::execute_store_shared_heap_scalar::<4>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreSharedHeap64 => {
                $step!(super::execute_store_shared_heap_scalar::<8>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreRaw8 => {
                $step!(super::execute_store_raw_scalar::<1>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreRaw16 => {
                $step!(super::execute_store_raw_scalar::<2>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreRaw32 => {
                $step!(super::execute_store_raw_scalar::<4>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreRaw64 => {
                $step!(super::execute_store_raw_scalar::<8>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreStack8 => $step!(super::execute_store_stack_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreStack16 => {
                $step!(super::execute_store_stack_scalar::<2>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreStack32 => {
                $step!(super::execute_store_stack_scalar::<4>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreStack64 => {
                $step!(super::execute_store_stack_scalar::<8>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreFrame8 => $step!(super::execute_store_frame_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreFrame16 => {
                $step!(super::execute_store_frame_scalar::<2>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreFrame32 => {
                $step!(super::execute_store_frame_scalar::<4>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreFrame64 => {
                $step!(super::execute_store_frame_scalar::<8>(
                    $activation,
                    instruction
                ))
            }
            Op::StoreFrameValue8 => $step!(super::execute_store_frame_value_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreFrameValue16 => $step!(super::execute_store_frame_value_scalar::<2>(
                $activation,
                instruction
            )),
            Op::StoreFrameValue32 => $step!(super::execute_store_frame_value_scalar::<4>(
                $activation,
                instruction
            )),
            Op::StoreFrameValue64 => $step!(super::execute_store_frame_value_scalar::<8>(
                $activation,
                instruction
            )),
            Op::StoreStatic8 => $step!(super::execute_store_static_scalar::<1>(
                $activation,
                instruction
            )),
            Op::StoreStatic16 => $step!(super::execute_store_static_scalar::<2>(
                $activation,
                instruction
            )),
            Op::StoreStatic32 => $step!(super::execute_store_static_scalar::<4>(
                $activation,
                instruction
            )),
            Op::StoreStatic64 => $step!(super::execute_store_static_scalar::<8>(
                $activation,
                instruction
            )),
            Op::Abort => $transfer!(super::execute_abort($activation, instruction)),
            Op::AddressFrameValueOffset => {
                $step!(super::execute_address_frame_value_offset(
                    $activation,
                    instruction
                ))
            }
            Op::AddressFrameValueElement => {
                $step!(super::execute_address_frame_value_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressHeapOffset => {
                $step!(super::execute_address_heap_offset($activation, instruction))
            }
            Op::AddressSharedHeapOffset => {
                $step!(super::execute_address_shared_heap_offset(
                    $activation,
                    instruction
                ))
            }
            Op::AddressRawOffset => {
                $step!(super::execute_address_raw_offset($activation, instruction))
            }
            Op::AddressStackOffset => {
                $step!(super::execute_address_stack_offset(
                    $activation,
                    instruction
                ))
            }
            Op::AddressFrameOffset => {
                $step!(super::execute_address_frame_offset(
                    $activation,
                    instruction
                ))
            }
            Op::AddressStaticOffset => {
                $step!(super::execute_address_static_offset(
                    $activation,
                    instruction
                ))
            }
            Op::AddressHeapElement => {
                $step!(super::execute_address_heap_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressSharedHeapElement => {
                $step!(super::execute_address_shared_heap_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressRawElement => {
                $step!(super::execute_address_raw_element($activation, instruction))
            }
            Op::AddressStackElement => {
                $step!(super::execute_address_stack_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressFrameElement => {
                $step!(super::execute_address_frame_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressStaticElement => {
                $step!(super::execute_address_static_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressHeapSliceElement => {
                $step!(super::execute_address_heap_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressSharedHeapSliceElement => {
                $step!(super::execute_address_shared_heap_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressRawSliceElement => {
                $step!(super::execute_address_raw_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressStackSliceElement => {
                $step!(super::execute_address_stack_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressFrameSliceElement => {
                $step!(super::execute_address_frame_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AddressStaticSliceElement => {
                $step!(super::execute_address_static_slice_element(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapZeroed => {
                $step!(super::execute_allocate_heap_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapUninit => {
                $step!(super::execute_allocate_heap_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallNoscanZeroed => {
                $step!(super::execute_allocate_heap_small_noscan_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallNoscanUninit => {
                $step!(super::execute_allocate_heap_small_noscan_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallScanZeroed => {
                $step!(super::execute_allocate_heap_small_scan_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallScanUninit => {
                $step!(super::execute_allocate_heap_small_scan_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallSharedEdgeZeroed => {
                $step!(super::execute_allocate_heap_small_shared_edge_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapSmallSharedEdgeUninit => {
                $step!(super::execute_allocate_heap_small_shared_edge_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapZeroed => {
                $step!(super::execute_allocate_shared_heap_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapUninit => {
                $step!(super::execute_allocate_shared_heap_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapSmallZeroed => {
                $step!(super::execute_allocate_shared_heap_small_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapSmallUninit => {
                $step!(super::execute_allocate_shared_heap_small_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapZeroedBranch => {
                $transfer!(super::execute_allocate_heap_zeroed_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateHeapUninitBranch => {
                $transfer!(super::execute_allocate_heap_uninit_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapZeroedBranch => {
                $transfer!(super::execute_allocate_shared_heap_zeroed_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedHeapUninitBranch => {
                $transfer!(super::execute_allocate_shared_heap_uninit_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSliceZeroed => {
                $step!(super::execute_allocate_slice_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSliceUninit => {
                $step!(super::execute_allocate_slice_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedSliceZeroed => {
                $step!(super::execute_allocate_shared_slice_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedSliceUninit => {
                $step!(super::execute_allocate_shared_slice_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSliceZeroedBranch => {
                $transfer!(super::execute_allocate_slice_zeroed_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSliceUninitBranch => {
                $transfer!(super::execute_allocate_slice_uninit_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedSliceZeroedBranch => {
                $transfer!(super::execute_allocate_shared_slice_zeroed_branch(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateSharedSliceUninitBranch => {
                $transfer!(super::execute_allocate_shared_slice_uninit_branch(
                    $activation,
                    instruction
                ))
            }
            Op::FreeHeap => $step!(super::execute_free_heap($activation, instruction)),
            Op::FreeSharedHeap => $step!(super::execute_free_shared_heap($activation, instruction)),
            Op::AllocateStackZeroed => {
                $step!(super::execute_allocate_stack_zeroed(
                    $activation,
                    instruction
                ))
            }
            Op::AllocateStackUninit => {
                $step!(super::execute_allocate_stack_uninit(
                    $activation,
                    instruction
                ))
            }
            Op::PinHeap => $step!(super::execute_pin_heap($activation, instruction)),
            Op::PinSharedHeap => $step!(super::execute_pin_shared_heap($activation, instruction)),
            Op::UnpinHeap => $step!(super::execute_unpin_heap($activation, instruction)),
            Op::UnpinSharedHeap => {
                $step!(super::execute_unpin_shared_heap($activation, instruction))
            }
            Op::VectorBinary => $step!(super::execute_vector_binary($activation, instruction)),
            Op::PackedAdd32x4 => $step!(super::execute_packed_add_32x4($activation, instruction)),
            Op::PackedSub32x4 => $step!(super::execute_packed_sub_32x4($activation, instruction)),
            Op::PackedMul32x4 => $step!(super::execute_packed_mul_32x4($activation, instruction)),
            Op::PackedAnd32x4 => $step!(super::execute_packed_and_32x4($activation, instruction)),
            Op::PackedOr32x4 => $step!(super::execute_packed_or_32x4($activation, instruction)),
            Op::PackedXor32x4 => $step!(super::execute_packed_xor_32x4($activation, instruction)),
            Op::PackedShl32x4 => $step!(super::execute_packed_shl_32x4($activation, instruction)),
            Op::PackedShrI32x4 => $step!(super::execute_packed_shr_i32x4($activation, instruction)),
            Op::PackedShrU32x4 => $step!(super::execute_packed_shr_u32x4($activation, instruction)),
            Op::PackedAdd64x2 => $step!(super::execute_packed_add_64x2($activation, instruction)),
            Op::PackedSub64x2 => $step!(super::execute_packed_sub_64x2($activation, instruction)),
            Op::PackedMul64x2 => $step!(super::execute_packed_mul_64x2($activation, instruction)),
            Op::PackedAnd64x2 => $step!(super::execute_packed_and_64x2($activation, instruction)),
            Op::PackedOr64x2 => $step!(super::execute_packed_or_64x2($activation, instruction)),
            Op::PackedXor64x2 => $step!(super::execute_packed_xor_64x2($activation, instruction)),
            Op::PackedShl64x2 => $step!(super::execute_packed_shl_64x2($activation, instruction)),
            Op::PackedShrI64x2 => $step!(super::execute_packed_shr_i64x2($activation, instruction)),
            Op::PackedShrU64x2 => $step!(super::execute_packed_shr_u64x2($activation, instruction)),
            Op::PackedAddF32x4 => $step!(super::execute_packed_add_f32x4($activation, instruction)),
            Op::PackedSubF32x4 => $step!(super::execute_packed_sub_f32x4($activation, instruction)),
            Op::PackedMulF32x4 => $step!(super::execute_packed_mul_f32x4($activation, instruction)),
            Op::PackedDivF32x4 => $step!(super::execute_packed_div_f32x4($activation, instruction)),
            Op::PackedAddF64x2 => $step!(super::execute_packed_add_f64x2($activation, instruction)),
            Op::PackedSubF64x2 => $step!(super::execute_packed_sub_f64x2($activation, instruction)),
            Op::PackedMulF64x2 => $step!(super::execute_packed_mul_f64x2($activation, instruction)),
            Op::PackedDivF64x2 => $step!(super::execute_packed_div_f64x2($activation, instruction)),
            Op::TensorBinary => $step!(super::execute_tensor_binary($activation, instruction)),
            Op::TensorContiguousBinary => {
                $step!(super::execute_tensor_contiguous_binary(
                    $activation,
                    instruction
                ))
            }
            Op::AndBool => $step!(super::execute_and_bool($activation, instruction)),
            Op::OrBool => $step!(super::execute_or_bool($activation, instruction)),
            Op::XorBool => $step!(super::execute_xor_bool($activation, instruction)),
            Op::AddI32 => $step!(super::execute_add_i32($activation, instruction)),
            Op::AddU32 => $step!(super::execute_add_u32($activation, instruction)),
            Op::AddI64 => $step!(super::execute_add_i64($activation, instruction)),
            Op::AddU64 => $step!(super::execute_add_u64($activation, instruction)),
            Op::SubI32 => $step!(super::execute_sub_i32($activation, instruction)),
            Op::SubU32 => $step!(super::execute_sub_u32($activation, instruction)),
            Op::SubI64 => $step!(super::execute_sub_i64($activation, instruction)),
            Op::SubU64 => $step!(super::execute_sub_u64($activation, instruction)),
            Op::MulI32 => $step!(super::execute_mul_i32($activation, instruction)),
            Op::MulU32 => $step!(super::execute_mul_u32($activation, instruction)),
            Op::MulI64 => $step!(super::execute_mul_i64($activation, instruction)),
            Op::MulU64 => $step!(super::execute_mul_u64($activation, instruction)),
            Op::DivI32 => $step!(super::execute_div_i32($activation, instruction)),
            Op::DivU32 => $step!(super::execute_div_u32($activation, instruction)),
            Op::DivI64 => $step!(super::execute_div_i64($activation, instruction)),
            Op::DivU64 => $step!(super::execute_div_u64($activation, instruction)),
            Op::RemI32 => $step!(super::execute_rem_i32($activation, instruction)),
            Op::RemU32 => $step!(super::execute_rem_u32($activation, instruction)),
            Op::RemI64 => $step!(super::execute_rem_i64($activation, instruction)),
            Op::RemU64 => $step!(super::execute_rem_u64($activation, instruction)),
            Op::AddWordInt => $step!(super::execute_add_word_int($activation, instruction)),
            Op::AddWordUint => $step!(super::execute_add_word_uint($activation, instruction)),
            Op::SubWordInt => $step!(super::execute_sub_word_int($activation, instruction)),
            Op::SubWordUint => $step!(super::execute_sub_word_uint($activation, instruction)),
            Op::MulWordInt => $step!(super::execute_mul_word_int($activation, instruction)),
            Op::MulWordUint => $step!(super::execute_mul_word_uint($activation, instruction)),
            Op::DivWordInt => $step!(super::execute_div_word_int($activation, instruction)),
            Op::DivWordUint => $step!(super::execute_div_word_uint($activation, instruction)),
            Op::RemWordInt => $step!(super::execute_rem_word_int($activation, instruction)),
            Op::RemWordUint => $step!(super::execute_rem_word_uint($activation, instruction)),
            Op::And32 => $step!(super::execute_and_32($activation, instruction)),
            Op::And64 => $step!(super::execute_and_64($activation, instruction)),
            Op::Or32 => $step!(super::execute_or_32($activation, instruction)),
            Op::Or64 => $step!(super::execute_or_64($activation, instruction)),
            Op::Xor32 => $step!(super::execute_xor_32($activation, instruction)),
            Op::Xor64 => $step!(super::execute_xor_64($activation, instruction)),
            Op::Shl32 => $step!(super::execute_shl_32($activation, instruction)),
            Op::Shl64 => $step!(super::execute_shl_64($activation, instruction)),
            Op::ShrI32 => $step!(super::execute_shr_i32($activation, instruction)),
            Op::ShrU32 => $step!(super::execute_shr_u32($activation, instruction)),
            Op::ShrI64 => $step!(super::execute_shr_i64($activation, instruction)),
            Op::ShrU64 => $step!(super::execute_shr_u64($activation, instruction)),
            Op::AddWideInt => $step!(super::execute_add_wide_int($activation, instruction)),
            Op::SubWideInt => $step!(super::execute_sub_wide_int($activation, instruction)),
            Op::MulWideInt => $step!(super::execute_mul_wide_int($activation, instruction)),
            Op::DivWideInt => $step!(super::execute_div_wide_int($activation, instruction)),
            Op::DivWideUint => $step!(super::execute_div_wide_uint($activation, instruction)),
            Op::RemWideInt => $step!(super::execute_rem_wide_int($activation, instruction)),
            Op::RemWideUint => $step!(super::execute_rem_wide_uint($activation, instruction)),
            Op::AndWord => $step!(super::execute_and_word($activation, instruction)),
            Op::OrWord => $step!(super::execute_or_word($activation, instruction)),
            Op::XorWord => $step!(super::execute_xor_word($activation, instruction)),
            Op::ShlWord => $step!(super::execute_shl_word($activation, instruction)),
            Op::ShrWordInt => $step!(super::execute_shr_word_int($activation, instruction)),
            Op::ShrWordUint => $step!(super::execute_shr_word_uint($activation, instruction)),
            Op::AndWideInt => $step!(super::execute_and_wide_int($activation, instruction)),
            Op::OrWideInt => $step!(super::execute_or_wide_int($activation, instruction)),
            Op::XorWideInt => $step!(super::execute_xor_wide_int($activation, instruction)),
            Op::ShlWideInt => $step!(super::execute_shl_wide_int($activation, instruction)),
            Op::ShrWideInt => $step!(super::execute_shr_wide_int($activation, instruction)),
            Op::ShrWideUint => $step!(super::execute_shr_wide_uint($activation, instruction)),
            Op::AddF32 => $step!(super::execute_add_f32($activation, instruction)),
            Op::SubF32 => $step!(super::execute_sub_f32($activation, instruction)),
            Op::MulF32 => $step!(super::execute_mul_f32($activation, instruction)),
            Op::DivF32 => $step!(super::execute_div_f32($activation, instruction)),
            Op::EqF32 => $step!(super::execute_eq_f32($activation, instruction)),
            Op::NeF32 => $step!(super::execute_ne_f32($activation, instruction)),
            Op::LtF32 => $step!(super::execute_lt_f32($activation, instruction)),
            Op::LeF32 => $step!(super::execute_le_f32($activation, instruction)),
            Op::GtF32 => $step!(super::execute_gt_f32($activation, instruction)),
            Op::GeF32 => $step!(super::execute_ge_f32($activation, instruction)),
            Op::AddF64 => $step!(super::execute_add_f64($activation, instruction)),
            Op::SubF64 => $step!(super::execute_sub_f64($activation, instruction)),
            Op::MulF64 => $step!(super::execute_mul_f64($activation, instruction)),
            Op::DivF64 => $step!(super::execute_div_f64($activation, instruction)),
            Op::BinaryFloat => $step!(super::execute_binary_float($activation, instruction)),
            Op::EqF64 => $step!(super::execute_eq_f64($activation, instruction)),
            Op::NeF64 => $step!(super::execute_ne_f64($activation, instruction)),
            Op::LtF64 => $step!(super::execute_lt_f64($activation, instruction)),
            Op::LeF64 => $step!(super::execute_le_f64($activation, instruction)),
            Op::GtF64 => $step!(super::execute_gt_f64($activation, instruction)),
            Op::GeF64 => $step!(super::execute_ge_f64($activation, instruction)),
            Op::Eq32 => $step!(super::execute_eq_32($activation, instruction)),
            Op::Eq64 => $step!(super::execute_eq_64($activation, instruction)),
            Op::Ne32 => $step!(super::execute_ne_32($activation, instruction)),
            Op::Ne64 => $step!(super::execute_ne_64($activation, instruction)),
            Op::LtI32 => $step!(super::execute_lt_i32($activation, instruction)),
            Op::LtU32 => $step!(super::execute_lt_u32($activation, instruction)),
            Op::LtI64 => $step!(super::execute_lt_i64($activation, instruction)),
            Op::LtU64 => $step!(super::execute_lt_u64($activation, instruction)),
            Op::LeI32 => $step!(super::execute_le_i32($activation, instruction)),
            Op::LeU32 => $step!(super::execute_le_u32($activation, instruction)),
            Op::LeI64 => $step!(super::execute_le_i64($activation, instruction)),
            Op::LeU64 => $step!(super::execute_le_u64($activation, instruction)),
            Op::GtI32 => $step!(super::execute_gt_i32($activation, instruction)),
            Op::GtU32 => $step!(super::execute_gt_u32($activation, instruction)),
            Op::GtI64 => $step!(super::execute_gt_i64($activation, instruction)),
            Op::GtU64 => $step!(super::execute_gt_u64($activation, instruction)),
            Op::GeI32 => $step!(super::execute_ge_i32($activation, instruction)),
            Op::GeU32 => $step!(super::execute_ge_u32($activation, instruction)),
            Op::GeI64 => $step!(super::execute_ge_i64($activation, instruction)),
            Op::GeU64 => $step!(super::execute_ge_u64($activation, instruction)),
            Op::EqWord => $step!(super::execute_eq_word($activation, instruction)),
            Op::NeWord => $step!(super::execute_ne_word($activation, instruction)),
            Op::LtWordInt => $step!(super::execute_lt_word_int($activation, instruction)),
            Op::LtWordUint => $step!(super::execute_lt_word_uint($activation, instruction)),
            Op::LeWordInt => $step!(super::execute_le_word_int($activation, instruction)),
            Op::LeWordUint => $step!(super::execute_le_word_uint($activation, instruction)),
            Op::GtWordInt => $step!(super::execute_gt_word_int($activation, instruction)),
            Op::GtWordUint => $step!(super::execute_gt_word_uint($activation, instruction)),
            Op::GeWordInt => $step!(super::execute_ge_word_int($activation, instruction)),
            Op::GeWordUint => $step!(super::execute_ge_word_uint($activation, instruction)),
            Op::EqWideInt => $step!(super::execute_eq_wide_int($activation, instruction)),
            Op::NeWideInt => $step!(super::execute_ne_wide_int($activation, instruction)),
            Op::LtWideInt => $step!(super::execute_lt_wide_int($activation, instruction)),
            Op::LtWideUint => $step!(super::execute_lt_wide_uint($activation, instruction)),
            Op::LeWideInt => $step!(super::execute_le_wide_int($activation, instruction)),
            Op::LeWideUint => $step!(super::execute_le_wide_uint($activation, instruction)),
            Op::GtWideInt => $step!(super::execute_gt_wide_int($activation, instruction)),
            Op::GtWideUint => $step!(super::execute_gt_wide_uint($activation, instruction)),
            Op::GeWideInt => $step!(super::execute_ge_wide_int($activation, instruction)),
            Op::GeWideUint => $step!(super::execute_ge_wide_uint($activation, instruction)),
            Op::NegI32 => $step!(super::execute_neg_i32($activation, instruction)),
            Op::NegI64 => $step!(super::execute_neg_i64($activation, instruction)),
            Op::Not32 => $step!(super::execute_not_32($activation, instruction)),
            Op::Not64 => $step!(super::execute_not_64($activation, instruction)),
            Op::NegWordInt => $step!(super::execute_neg_word_int($activation, instruction)),
            Op::NotWord => $step!(super::execute_not_word($activation, instruction)),
            Op::NegWideInt => $step!(super::execute_neg_wide_int($activation, instruction)),
            Op::NotWideInt => $step!(super::execute_not_wide_int($activation, instruction)),
            Op::NegF32 => $step!(super::execute_neg_f32($activation, instruction)),
            Op::NegF64 => $step!(super::execute_neg_f64($activation, instruction)),
            Op::UnaryFloat => $step!(super::execute_unary_float($activation, instruction)),
            Op::NotBool => $step!(super::execute_not_bool($activation, instruction)),
            Op::VectorUnary => $step!(super::execute_vector_unary($activation, instruction)),
            Op::TensorUnary => $step!(super::execute_tensor_unary($activation, instruction)),
            Op::TensorContiguousUnary => {
                $step!(super::execute_tensor_contiguous_unary(
                    $activation,
                    instruction
                ))
            }
            Op::PackedNegI32x4 => $step!(super::execute_packed_neg_i32x4($activation, instruction)),
            Op::PackedNot32x4 => $step!(super::execute_packed_not_32x4($activation, instruction)),
            Op::PackedNegI64x2 => $step!(super::execute_packed_neg_i64x2($activation, instruction)),
            Op::PackedNot64x2 => $step!(super::execute_packed_not_64x2($activation, instruction)),
            Op::PackedNegF32x4 => $step!(super::execute_packed_neg_f32x4($activation, instruction)),
            Op::PackedNegF64x2 => $step!(super::execute_packed_neg_f64x2($activation, instruction)),
            Op::CastBitcast => $step!(super::execute_cast_bitcast($activation, instruction)),
            Op::CastTruncate => $step!(super::execute_cast_truncate($activation, instruction)),
            Op::CastZeroExtend => $step!(super::execute_cast_zero_extend($activation, instruction)),
            Op::CastSignExtend => $step!(super::execute_cast_sign_extend($activation, instruction)),
            Op::CastFloatToSignedInt => $step!(super::execute_cast_float_to_signed_int(
                $activation,
                instruction
            )),
            Op::CastFloatToUnsignedInt => {
                $step!({ super::execute_cast_float_to_unsigned_int($activation, instruction) })
            }
            Op::CastFloatToSignedIntSaturating => {
                $step!({
                    super::execute_cast_float_to_signed_int_saturating($activation, instruction)
                })
            }
            Op::CastFloatToUnsignedIntSaturating => $step!({
                super::execute_cast_float_to_unsigned_int_saturating($activation, instruction)
            }),
            Op::CastSignedIntToFloat => {
                $step!(super::execute_cast_signed_int_to_float(
                    $activation,
                    instruction
                ))
            }
            Op::CastUnsignedIntToFloat => $step!(super::execute_cast_unsigned_int_to_float(
                $activation,
                instruction
            )),
            Op::CastFloatConvert => {
                $step!(super::execute_cast_float_convert($activation, instruction))
            }
            Op::CastPointerToInt => {
                $step!(super::execute_cast_pointer_to_int($activation, instruction))
            }
            Op::CastIntToPointer => {
                $step!(super::execute_cast_int_to_pointer($activation, instruction))
            }
            Op::CastWordToWideInt => {
                $step!(super::execute_cast_word_to_wide_int(
                    $activation,
                    instruction
                ))
            }
            Op::CastWideIntToWord => {
                $step!(super::execute_cast_wide_int_to_word(
                    $activation,
                    instruction
                ))
            }
            Op::CastWideInt => $step!(super::execute_cast_wide_int($activation, instruction)),
            Op::CastTensorView => $step!(super::execute_tensor_view_cast($activation, instruction)),
            Op::Call => $transfer!(super::execute_call($activation, instruction, $block_pc)),
            Op::CallBranch => $transfer!(super::execute_call_branch($activation, instruction)),
            Op::CallIndirect => {
                $transfer!(super::execute_call_indirect(
                    $activation,
                    instruction,
                    $block_pc
                ))
            }
            Op::CallClosure => {
                $transfer!(super::execute_call_closure(
                    $activation,
                    instruction,
                    $block_pc
                ))
            }
            Op::CallIndirectBranch => {
                $transfer!(super::execute_call_indirect_branch(
                    $activation,
                    instruction
                ))
            }
            Op::CallClosureBranch => {
                $transfer!(super::execute_call_closure_branch($activation, instruction))
            }
            Op::CallVirtualHeap => $transfer!(super::execute_call_virtual_heap(
                $activation,
                instruction,
                $block_pc
            )),
            Op::CallVirtualSharedHeap => $transfer!({
                super::execute_call_virtual_shared_heap($activation, instruction, $block_pc)
            }),
            Op::CallVirtualHeapBranch => {
                $transfer!(super::execute_call_virtual_heap_branch(
                    $activation,
                    instruction
                ))
            }
            Op::CallVirtualSharedHeapBranch => {
                $transfer!(super::execute_call_virtual_shared_heap_branch(
                    $activation,
                    instruction
                ))
            }
            Op::CallDynamicHeap => $transfer!(super::execute_call_dynamic_heap(
                $activation,
                instruction,
                $block_pc
            )),
            Op::CallDynamicSharedHeap => $transfer!({
                super::execute_call_dynamic_shared_heap($activation, instruction, $block_pc)
            }),
            Op::CallDynamicHeapBranch => {
                $transfer!(super::execute_call_dynamic_heap_branch(
                    $activation,
                    instruction
                ))
            }
            Op::CallDynamicSharedHeapBranch => {
                $transfer!(super::execute_call_dynamic_shared_heap_branch(
                    $activation,
                    instruction
                ))
            }
            Op::TailCall => $transfer!(super::execute_tail_call($activation, instruction)),
            Op::TailCallSelf => $transfer!(super::execute_tail_call_self($activation, instruction)),
            Op::TailCallIndirect => {
                $transfer!(super::execute_tail_call_indirect($activation, instruction))
            }
            Op::TailCallClosure => {
                $transfer!(super::execute_tail_call_closure($activation, instruction))
            }
            Op::TailCallVirtualHeap => {
                $transfer!(super::execute_tail_call_virtual_heap(
                    $activation,
                    instruction
                ))
            }
            Op::TailCallVirtualSharedHeap => {
                $transfer!(super::execute_tail_call_virtual_shared_heap(
                    $activation,
                    instruction
                ))
            }
            Op::TailCallDynamicHeap => {
                $transfer!(super::execute_tail_call_dynamic_heap(
                    $activation,
                    instruction
                ))
            }
            Op::TailCallDynamicSharedHeap => {
                $transfer!(super::execute_tail_call_dynamic_shared_heap(
                    $activation,
                    instruction
                ))
            }
            Op::Jump => $transfer!(super::execute_jump($activation, instruction)),
            Op::BranchBool => $transfer!(super::execute_branch_bool($activation, instruction)),
            Op::BranchEq32 => $transfer!(super::execute_branch_eq_32($activation, instruction)),
            Op::BranchEq64 => $transfer!(super::execute_branch_eq_64($activation, instruction)),
            Op::BranchNe32 => $transfer!(super::execute_branch_ne_32($activation, instruction)),
            Op::BranchNe64 => $transfer!(super::execute_branch_ne_64($activation, instruction)),
            Op::BranchLtI32 => $transfer!(super::execute_branch_lt_i32($activation, instruction)),
            Op::BranchLtU32 => $transfer!(super::execute_branch_lt_u32($activation, instruction)),
            Op::BranchLtI64 => $transfer!(super::execute_branch_lt_i64($activation, instruction)),
            Op::BranchLtU64 => $transfer!(super::execute_branch_lt_u64($activation, instruction)),
            Op::BranchLeI32 => $transfer!(super::execute_branch_le_i32($activation, instruction)),
            Op::BranchLeU32 => $transfer!(super::execute_branch_le_u32($activation, instruction)),
            Op::BranchLeI64 => $transfer!(super::execute_branch_le_i64($activation, instruction)),
            Op::BranchLeU64 => $transfer!(super::execute_branch_le_u64($activation, instruction)),
            Op::BranchGtI32 => $transfer!(super::execute_branch_gt_i32($activation, instruction)),
            Op::BranchGtU32 => $transfer!(super::execute_branch_gt_u32($activation, instruction)),
            Op::BranchGtI64 => $transfer!(super::execute_branch_gt_i64($activation, instruction)),
            Op::BranchGtU64 => $transfer!(super::execute_branch_gt_u64($activation, instruction)),
            Op::BranchGeI32 => $transfer!(super::execute_branch_ge_i32($activation, instruction)),
            Op::BranchGeU32 => $transfer!(super::execute_branch_ge_u32($activation, instruction)),
            Op::BranchGeI64 => $transfer!(super::execute_branch_ge_i64($activation, instruction)),
            Op::BranchGeU64 => $transfer!(super::execute_branch_ge_u64($activation, instruction)),
            Op::BranchEqWord => $transfer!(super::execute_branch_eq_word($activation, instruction)),
            Op::BranchNeWord => $transfer!(super::execute_branch_ne_word($activation, instruction)),
            Op::BranchLtWordInt => {
                $transfer!(super::execute_branch_lt_word_int($activation, instruction))
            }
            Op::BranchLeWordInt => {
                $transfer!(super::execute_branch_le_word_int($activation, instruction))
            }
            Op::BranchGtWordInt => {
                $transfer!(super::execute_branch_gt_word_int($activation, instruction))
            }
            Op::BranchGeWordInt => {
                $transfer!(super::execute_branch_ge_word_int($activation, instruction))
            }
            Op::BranchLtWordUint => {
                $transfer!(super::execute_branch_lt_word_uint($activation, instruction))
            }
            Op::BranchLeWordUint => {
                $transfer!(super::execute_branch_le_word_uint($activation, instruction))
            }
            Op::BranchGtWordUint => {
                $transfer!(super::execute_branch_gt_word_uint($activation, instruction))
            }
            Op::BranchGeWordUint => {
                $transfer!(super::execute_branch_ge_word_uint($activation, instruction))
            }
            Op::BranchEqF32 => $transfer!(super::execute_branch_eq_f32($activation, instruction)),
            Op::BranchNeF32 => $transfer!(super::execute_branch_ne_f32($activation, instruction)),
            Op::BranchLtF32 => $transfer!(super::execute_branch_lt_f32($activation, instruction)),
            Op::BranchLeF32 => $transfer!(super::execute_branch_le_f32($activation, instruction)),
            Op::BranchGtF32 => $transfer!(super::execute_branch_gt_f32($activation, instruction)),
            Op::BranchGeF32 => $transfer!(super::execute_branch_ge_f32($activation, instruction)),
            Op::BranchEqF64 => $transfer!(super::execute_branch_eq_f64($activation, instruction)),
            Op::BranchNeF64 => $transfer!(super::execute_branch_ne_f64($activation, instruction)),
            Op::BranchLtF64 => $transfer!(super::execute_branch_lt_f64($activation, instruction)),
            Op::BranchLeF64 => $transfer!(super::execute_branch_le_f64($activation, instruction)),
            Op::BranchGtF64 => $transfer!(super::execute_branch_gt_f64($activation, instruction)),
            Op::BranchGeF64 => $transfer!(super::execute_branch_ge_f64($activation, instruction)),
            Op::Switch => $transfer!(super::execute_switch($activation, instruction)),
            Op::SwitchTable => $transfer!(super::execute_switch_table($activation, instruction)),
            Op::Check => $transfer!(super::execute_check($activation, instruction)),
            Op::Assume => $step!(super::execute_assume($activation, instruction)),
            Op::BarrierWriteHeap => {
                $step!(super::execute_barrier_write_heap($activation, instruction))
            }
            Op::BarrierWriteSharedHeap => {
                $step!(super::execute_barrier_write_shared_heap(
                    $activation,
                    instruction
                ))
            }
            Op::AtomicLoad => $step!(super::execute_atomic_load($activation, instruction)),
            Op::AtomicStore => $step!(super::execute_atomic_store($activation, instruction)),
            Op::AtomicExchange => $step!(super::execute_atomic_exchange($activation, instruction)),
            Op::AtomicCompareExchange => {
                $step!(super::execute_atomic_compare_exchange(
                    $activation,
                    instruction
                ))
            }
            Op::AtomicReadModifyWrite => {
                $step!(super::execute_atomic_read_modify_write(
                    $activation,
                    instruction
                ))
            }
            Op::AtomicFence => $step!(super::execute_atomic_fence($activation, instruction)),
            Op::Intrinsic => $step!(super::execute_intrinsic($activation, instruction)),
            Op::ReturnWord => $transfer!(super::execute_return_word($activation, instruction)),
            Op::ReturnAddress => {
                $transfer!(super::execute_return_address($activation, instruction))
            }
            Op::ReturnVoid => $transfer!(super::execute_return_void($activation, instruction)),
            Op::TensorBroadcast => {
                $step!(super::execute_tensor_broadcast($activation, instruction))
            }
            Op::TensorCast => $step!(super::execute_tensor_cast($activation, instruction)),
            Op::TensorConcat => $step!(super::execute_tensor_concat($activation, instruction)),
            Op::TensorConvert => $step!(super::execute_tensor_convert($activation, instruction)),
            Op::TensorConvolution => {
                $step!(super::execute_tensor_convolution($activation, instruction))
            }
            Op::TensorCopy => $step!(super::execute_tensor_copy($activation, instruction)),
            Op::TensorDot => $step!(super::execute_tensor_dot($activation, instruction)),
            Op::TensorFill => $step!(super::execute_tensor_fill($activation, instruction)),
            Op::TensorGather => $step!(super::execute_tensor_gather($activation, instruction)),
            Op::TensorExtract => $step!(super::execute_tensor_extract($activation, instruction)),
            Op::TensorIndexReduce => {
                $step!(super::execute_tensor_index_reduce($activation, instruction))
            }
            Op::TensorLoad => $step!(super::execute_tensor_load($activation, instruction)),
            Op::TensorSplat => $step!(super::execute_tensor_splat($activation, instruction)),
            Op::TensorPad => $step!(super::execute_tensor_pad($activation, instruction)),
            Op::TensorReduce => $step!(super::execute_tensor_reduce($activation, instruction)),
            Op::TensorReshape => $step!(super::execute_tensor_reshape($activation, instruction)),
            Op::TensorScatter => $step!(super::execute_tensor_scatter($activation, instruction)),
            Op::TensorSelect => $step!(super::execute_tensor_select($activation, instruction)),
            Op::TensorSlice => $step!(super::execute_tensor_slice($activation, instruction)),
            Op::TensorStore => $step!(super::execute_tensor_store($activation, instruction)),
            Op::TensorTranspose => {
                $step!(super::execute_tensor_transpose($activation, instruction))
            }
            Op::TensorView => $step!(super::execute_tensor_view($activation, instruction)),
            Op::Panic | Op::PanicValue | Op::ResumePanic => {
                $transfer!(super::execute_panic($activation, instruction))
            }
            Op::Unreachable => $transfer!(super::execute_unreachable($activation, instruction)),
            Op::VectorConvert => $step!(super::execute_vector_convert($activation, instruction)),
            Op::VectorExtract => $step!(super::execute_vector_extract($activation, instruction)),
            Op::VectorInsert => $step!(super::execute_vector_insert($activation, instruction)),
            Op::VectorReduce => $step!(super::execute_vector_reduce($activation, instruction)),
            Op::VectorSelect => $step!(super::execute_vector_select($activation, instruction)),
            Op::VectorShuffle => $step!(super::execute_vector_shuffle($activation, instruction)),
            Op::VectorSplat => $step!(super::execute_vector_splat($activation, instruction)),
            Op::PackedSplat32x4 => {
                $step!(super::execute_packed_splat_32x4($activation, instruction))
            }
            Op::PackedSplat64x2 => {
                $step!(super::execute_packed_splat_64x2($activation, instruction))
            }
            Op::YieldWord => $transfer!(super::execute_yield_word($activation, instruction)),
            Op::YieldAddress => $transfer!(super::execute_yield_address($activation, instruction)),
        }
    };
}

/// Dispatch one block until it produces a control transfer.
pub(crate) fn dispatch_block(
    activation: &mut Activation<'_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> Transfer {
    dispatch_block_inner(activation, function, block_index, pc)
}

/// Dispatch one block in the trusted lowered-code VM.
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
fn dispatch_block_inner(
    activation: &mut Activation<'_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> Transfer {
    let program = activation.machine.program.clone();
    let mut function = function;

    // start at the requested block offset
    let (mut block_start, mut pc, mut block_end) = match block_bounds(function, block_index) {
        Ok((block_start, block_end)) => (block_start, block_start + pc, block_end),
        Err(error) => return Transfer::Error(error),
    };

    loop {
        // guard against malformed block metadata
        if pc >= block_end {
            return Transfer::Error(Error::invalid_instruction());
        }

        macro_rules! step {
            ($operation:expr) => {{
                if let Err(error) = $operation {
                    return Transfer::Error(error);
                }

                pc += 1;
                continue;
            }};
        }

        macro_rules! transfer {
            ($operation:expr) => {{
                match $operation {
                    Transfer::Jump {
                        block: target,
                        moves,
                    } => {
                        (block_start, pc, block_end) =
                            match enter_block(activation, function, target, moves) {
                                Ok((block_start, block_end)) => {
                                    (block_start, block_start, block_end)
                                }
                                Err(error) => return Transfer::Error(error),
                            };
                        continue;
                    }
                    Transfer::Enter => {
                        let frame = activation.active_frame();
                        let function_id = frame.function();
                        let block = frame.block;
                        let Some(next_function) = program.functions.function_by_id(function_id)
                        else {
                            return Transfer::Error(Error::undefined_function(function_id));
                        };
                        function = next_function;
                        (block_start, pc, block_end) = match block_bounds(function, block) {
                            Ok((block_start, block_end)) => (block_start, block_start, block_end),
                            Err(error) => return Transfer::Error(error),
                        };
                        continue;
                    }
                    transfer => return transfer,
                }
            }};
        }

        dispatch_instruction!(activation, function, pc, pc - block_start, step, transfer);
    }
}

/// Dispatch one block and count executed instructions.
pub(crate) fn dispatch_block_counted(
    activation: &mut Activation<'_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> BlockDispatch {
    dispatch_block_counted_inner(activation, function, block_index, pc)
}

/// Dispatch one counted block in the trusted lowered-code VM.
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
fn dispatch_block_counted_inner(
    activation: &mut Activation<'_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> BlockDispatch {
    let (block_start, mut pc, block_end) = match block_bounds(function, block_index) {
        Ok((block_start, block_end)) => (block_start, block_start + pc, block_end),
        Err(error) => {
            return BlockDispatch {
                transfer: Transfer::Error(error),
                executed: 0,
            };
        }
    };
    let mut executed = 0;

    loop {
        if pc >= block_end {
            return BlockDispatch {
                transfer: Transfer::Error(Error::invalid_instruction()),
                executed,
            };
        }

        executed += 1;
        macro_rules! step {
            ($operation:expr) => {{
                if let Err(error) = $operation {
                    return BlockDispatch {
                        transfer: Transfer::Error(error),
                        executed,
                    };
                }

                pc += 1;
                continue;
            }};
        }

        macro_rules! transfer {
            ($operation:expr) => {{
                return BlockDispatch {
                    transfer: $operation,
                    executed,
                };
            }};
        }

        dispatch_instruction!(activation, function, pc, pc - block_start, step, transfer);
    }
}
