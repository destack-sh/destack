use crate::diagnostic::Error;
use crate::interpreter::Machine;
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
    machine: &mut Machine<'_, '_>,
    function: &Function,
    block: u32,
    moves: MoveRange,
) -> Result<(usize, usize), Error> {
    let move_pool = function.move_pool.as_slice();

    // move block parameters before retargeting the frame
    let frame = machine.active_frame_mut();
    move_values_within_frame(frame, moves, move_pool)?;

    // retarget the frame to the destination block
    let frame = machine.active_frame_mut();
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
    ($machine:ident, $function:ident, $pc:ident, $block_pc:expr, $step:ident, $transfer:ident) => {
        let instruction = &$function.code[$pc];

        match instruction.op {
            Op::LoadConstWord => $step!(super::execute_load_const_word($machine, instruction)),
            Op::LoadConstBytes => $step!(super::execute_load_const_bytes($machine, instruction)),
            Op::MoveWord => $step!(super::execute_move_word($machine, instruction)),
            Op::MoveFrame => $step!(super::execute_move_frame($machine, instruction)),
            Op::LoadHeapBytes => {
                $step!(super::execute_load_heap_bytes($machine, instruction))
            }
            Op::LoadSharedHeapBytes => {
                $step!(super::execute_load_shared_heap_bytes($machine, instruction))
            }
            Op::LoadRawBytes => {
                $step!(super::execute_load_raw_bytes($machine, instruction))
            }
            Op::LoadStackBytes => {
                $step!(super::execute_load_stack_bytes($machine, instruction))
            }
            Op::LoadFrameBytes => {
                $step!(super::execute_load_frame_bytes($machine, instruction))
            }
            Op::LoadStaticBytes => {
                $step!(super::execute_load_static_bytes($machine, instruction))
            }
            Op::StoreHeapBytes => {
                $step!(super::execute_store_heap_bytes($machine, instruction))
            }
            Op::StoreSharedHeapBytes => {
                $step!(super::execute_store_shared_heap_bytes(
                    $machine,
                    instruction
                ))
            }
            Op::StoreRawBytes => {
                $step!(super::execute_store_raw_bytes($machine, instruction))
            }
            Op::StoreStackBytes => {
                $step!(super::execute_store_stack_bytes($machine, instruction))
            }
            Op::StoreFrameBytes => {
                $step!(super::execute_store_frame_bytes($machine, instruction))
            }
            Op::StoreStaticBytes => {
                $step!(super::execute_store_static_bytes($machine, instruction))
            }
            Op::SelectWord => $step!(super::execute_select_word($machine, instruction)),
            Op::SelectFrame => $step!(super::execute_select_frame($machine, instruction)),
            Op::AddressLocal => $step!(super::execute_address_local($machine, instruction)),
            Op::AddressStatic => $step!(super::execute_address_static($machine, instruction)),
            Op::AddressFunction => $step!(super::execute_address_function($machine, instruction)),
            Op::BindCallableWord => {
                $step!(super::execute_bind_callable_word($machine, instruction))
            }
            Op::BindCallableAddress => {
                $step!(super::execute_bind_callable_address($machine, instruction))
            }
            Op::LoadCallableEnvironment => {
                $step!({ super::execute_load_callable_environment($machine, instruction) })
            }
            Op::LoadHeapU8 => $step!(super::execute_load_heap_scalar::<1, false>(
                $machine,
                instruction
            )),
            Op::LoadHeapI8 => $step!(super::execute_load_heap_scalar::<1, true>(
                $machine,
                instruction
            )),
            Op::LoadHeapU16 => $step!(super::execute_load_heap_scalar::<2, false>(
                $machine,
                instruction
            )),
            Op::LoadHeapI16 => $step!(super::execute_load_heap_scalar::<2, true>(
                $machine,
                instruction
            )),
            Op::LoadHeapU32 => $step!(super::execute_load_heap_scalar::<4, false>(
                $machine,
                instruction
            )),
            Op::LoadHeapI32 => $step!(super::execute_load_heap_scalar::<4, true>(
                $machine,
                instruction
            )),
            Op::LoadHeap64 => $step!(super::execute_load_heap_scalar::<8, false>(
                $machine,
                instruction
            )),
            Op::LoadSharedHeapU8 => {
                $step!(super::execute_load_shared_heap_scalar::<1, false>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeapI8 => {
                $step!(super::execute_load_shared_heap_scalar::<1, true>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeapU16 => {
                $step!(super::execute_load_shared_heap_scalar::<2, false>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeapI16 => {
                $step!(super::execute_load_shared_heap_scalar::<2, true>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeapU32 => {
                $step!(super::execute_load_shared_heap_scalar::<4, false>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeapI32 => {
                $step!(super::execute_load_shared_heap_scalar::<4, true>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadSharedHeap64 => {
                $step!(super::execute_load_shared_heap_scalar::<8, false>(
                    $machine,
                    instruction
                ))
            }
            Op::LoadRawU8 => $step!(super::execute_load_raw_scalar::<1, false>(
                $machine,
                instruction
            )),
            Op::LoadRawI8 => $step!(super::execute_load_raw_scalar::<1, true>(
                $machine,
                instruction
            )),
            Op::LoadRawU16 => $step!(super::execute_load_raw_scalar::<2, false>(
                $machine,
                instruction
            )),
            Op::LoadRawI16 => $step!(super::execute_load_raw_scalar::<2, true>(
                $machine,
                instruction
            )),
            Op::LoadRawU32 => $step!(super::execute_load_raw_scalar::<4, false>(
                $machine,
                instruction
            )),
            Op::LoadRawI32 => $step!(super::execute_load_raw_scalar::<4, true>(
                $machine,
                instruction
            )),
            Op::LoadRaw64 => $step!(super::execute_load_raw_scalar::<8, false>(
                $machine,
                instruction
            )),
            Op::LoadStackU8 => $step!(super::execute_load_stack_scalar::<1, false>(
                $machine,
                instruction
            )),
            Op::LoadStackI8 => $step!(super::execute_load_stack_scalar::<1, true>(
                $machine,
                instruction
            )),
            Op::LoadStackU16 => $step!(super::execute_load_stack_scalar::<2, false>(
                $machine,
                instruction
            )),
            Op::LoadStackI16 => $step!(super::execute_load_stack_scalar::<2, true>(
                $machine,
                instruction
            )),
            Op::LoadStackU32 => $step!(super::execute_load_stack_scalar::<4, false>(
                $machine,
                instruction
            )),
            Op::LoadStackI32 => $step!(super::execute_load_stack_scalar::<4, true>(
                $machine,
                instruction
            )),
            Op::LoadStack64 => $step!(super::execute_load_stack_scalar::<8, false>(
                $machine,
                instruction
            )),
            Op::LoadFrameU8 => $step!(super::execute_load_frame_scalar::<1, false>(
                $machine,
                instruction
            )),
            Op::LoadFrameI8 => $step!(super::execute_load_frame_scalar::<1, true>(
                $machine,
                instruction
            )),
            Op::LoadFrameU16 => $step!(super::execute_load_frame_scalar::<2, false>(
                $machine,
                instruction
            )),
            Op::LoadFrameI16 => $step!(super::execute_load_frame_scalar::<2, true>(
                $machine,
                instruction
            )),
            Op::LoadFrameU32 => $step!(super::execute_load_frame_scalar::<4, false>(
                $machine,
                instruction
            )),
            Op::LoadFrameI32 => $step!(super::execute_load_frame_scalar::<4, true>(
                $machine,
                instruction
            )),
            Op::LoadFrame64 => $step!(super::execute_load_frame_scalar::<8, false>(
                $machine,
                instruction
            )),
            Op::LoadFrameValueU8 => {
                $step!({
                    super::execute_load_frame_value_scalar::<1, false>($machine, instruction)
                })
            }
            Op::LoadFrameValueI8 => {
                $step!({ super::execute_load_frame_value_scalar::<1, true>($machine, instruction) })
            }
            Op::LoadFrameValueU16 => {
                $step!({
                    super::execute_load_frame_value_scalar::<2, false>($machine, instruction)
                })
            }
            Op::LoadFrameValueI16 => {
                $step!({ super::execute_load_frame_value_scalar::<2, true>($machine, instruction) })
            }
            Op::LoadFrameValueU32 => {
                $step!({
                    super::execute_load_frame_value_scalar::<4, false>($machine, instruction)
                })
            }
            Op::LoadFrameValueI32 => {
                $step!({ super::execute_load_frame_value_scalar::<4, true>($machine, instruction) })
            }
            Op::LoadFrameValue64 => {
                $step!({
                    super::execute_load_frame_value_scalar::<8, false>($machine, instruction)
                })
            }
            Op::LoadStaticU8 => $step!(super::execute_load_static_scalar::<1, false>(
                $machine,
                instruction
            )),
            Op::LoadStaticI8 => $step!(super::execute_load_static_scalar::<1, true>(
                $machine,
                instruction
            )),
            Op::LoadStaticU16 => $step!(super::execute_load_static_scalar::<2, false>(
                $machine,
                instruction
            )),
            Op::LoadStaticI16 => $step!(super::execute_load_static_scalar::<2, true>(
                $machine,
                instruction
            )),
            Op::LoadStaticU32 => $step!(super::execute_load_static_scalar::<4, false>(
                $machine,
                instruction
            )),
            Op::LoadStaticI32 => $step!(super::execute_load_static_scalar::<4, true>(
                $machine,
                instruction
            )),
            Op::LoadStatic64 => $step!(super::execute_load_static_scalar::<8, false>(
                $machine,
                instruction
            )),
            Op::StoreHeap8 => $step!(super::execute_store_heap_scalar::<1>($machine, instruction)),
            Op::StoreHeap16 => $step!(super::execute_store_heap_scalar::<2>($machine, instruction)),
            Op::StoreHeap32 => $step!(super::execute_store_heap_scalar::<4>($machine, instruction)),
            Op::StoreHeap64 => $step!(super::execute_store_heap_scalar::<8>($machine, instruction)),
            Op::StoreSharedHeap8 => $step!(super::execute_store_shared_heap_scalar::<1>(
                $machine,
                instruction
            )),
            Op::StoreSharedHeap16 => {
                $step!(super::execute_store_shared_heap_scalar::<2>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreSharedHeap32 => {
                $step!(super::execute_store_shared_heap_scalar::<4>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreSharedHeap64 => {
                $step!(super::execute_store_shared_heap_scalar::<8>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreRaw8 => {
                $step!(super::execute_store_raw_scalar::<1>($machine, instruction))
            }
            Op::StoreRaw16 => {
                $step!(super::execute_store_raw_scalar::<2>($machine, instruction))
            }
            Op::StoreRaw32 => {
                $step!(super::execute_store_raw_scalar::<4>($machine, instruction))
            }
            Op::StoreRaw64 => {
                $step!(super::execute_store_raw_scalar::<8>($machine, instruction))
            }
            Op::StoreStack8 => $step!(super::execute_store_stack_scalar::<1>(
                $machine,
                instruction
            )),
            Op::StoreStack16 => {
                $step!(super::execute_store_stack_scalar::<2>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreStack32 => {
                $step!(super::execute_store_stack_scalar::<4>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreStack64 => {
                $step!(super::execute_store_stack_scalar::<8>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreFrame8 => $step!(super::execute_store_frame_scalar::<1>(
                $machine,
                instruction
            )),
            Op::StoreFrame16 => {
                $step!(super::execute_store_frame_scalar::<2>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreFrame32 => {
                $step!(super::execute_store_frame_scalar::<4>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreFrame64 => {
                $step!(super::execute_store_frame_scalar::<8>(
                    $machine,
                    instruction
                ))
            }
            Op::StoreFrameValue8 => $step!(super::execute_store_frame_value_scalar::<1>(
                $machine,
                instruction
            )),
            Op::StoreFrameValue16 => $step!(super::execute_store_frame_value_scalar::<2>(
                $machine,
                instruction
            )),
            Op::StoreFrameValue32 => $step!(super::execute_store_frame_value_scalar::<4>(
                $machine,
                instruction
            )),
            Op::StoreFrameValue64 => $step!(super::execute_store_frame_value_scalar::<8>(
                $machine,
                instruction
            )),
            Op::StoreStatic8 => $step!(super::execute_store_static_scalar::<1>(
                $machine,
                instruction
            )),
            Op::StoreStatic16 => $step!(super::execute_store_static_scalar::<2>(
                $machine,
                instruction
            )),
            Op::StoreStatic32 => $step!(super::execute_store_static_scalar::<4>(
                $machine,
                instruction
            )),
            Op::StoreStatic64 => $step!(super::execute_store_static_scalar::<8>(
                $machine,
                instruction
            )),
            Op::Abort => $transfer!(super::execute_abort($machine, instruction)),
            Op::AddressFrameValueOffset => {
                $step!(super::execute_address_frame_value_offset(
                    $machine,
                    instruction
                ))
            }
            Op::AddressFrameValueElement => {
                $step!(super::execute_address_frame_value_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressHeapOffset => {
                $step!(super::execute_address_heap_offset($machine, instruction))
            }
            Op::AddressSharedHeapOffset => {
                $step!(super::execute_address_shared_heap_offset(
                    $machine,
                    instruction
                ))
            }
            Op::AddressRawOffset => {
                $step!(super::execute_address_raw_offset($machine, instruction))
            }
            Op::AddressStackOffset => {
                $step!(super::execute_address_stack_offset($machine, instruction))
            }
            Op::AddressFrameOffset => {
                $step!(super::execute_address_frame_offset($machine, instruction))
            }
            Op::AddressStaticOffset => {
                $step!(super::execute_address_static_offset($machine, instruction))
            }
            Op::AddressHeapElement => {
                $step!(super::execute_address_heap_element($machine, instruction))
            }
            Op::AddressSharedHeapElement => {
                $step!(super::execute_address_shared_heap_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressRawElement => {
                $step!(super::execute_address_raw_element($machine, instruction))
            }
            Op::AddressStackElement => {
                $step!(super::execute_address_stack_element($machine, instruction))
            }
            Op::AddressFrameElement => {
                $step!(super::execute_address_frame_element($machine, instruction))
            }
            Op::AddressStaticElement => {
                $step!(super::execute_address_static_element($machine, instruction))
            }
            Op::AddressHeapSliceElement => {
                $step!(super::execute_address_heap_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressSharedHeapSliceElement => {
                $step!(super::execute_address_shared_heap_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressRawSliceElement => {
                $step!(super::execute_address_raw_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressStackSliceElement => {
                $step!(super::execute_address_stack_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressFrameSliceElement => {
                $step!(super::execute_address_frame_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AddressStaticSliceElement => {
                $step!(super::execute_address_static_slice_element(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapZeroed => {
                $step!(super::execute_allocate_heap_zeroed($machine, instruction))
            }
            Op::AllocateHeapUninit => {
                $step!(super::execute_allocate_heap_uninit($machine, instruction))
            }
            Op::AllocateHeapSmallNoscanZeroed => {
                $step!(super::execute_allocate_heap_small_noscan_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapSmallNoscanUninit => {
                $step!(super::execute_allocate_heap_small_noscan_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapSmallScanZeroed => {
                $step!(super::execute_allocate_heap_small_scan_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapSmallScanUninit => {
                $step!(super::execute_allocate_heap_small_scan_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapSmallSharedEdgeZeroed => {
                $step!(super::execute_allocate_heap_small_shared_edge_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapSmallSharedEdgeUninit => {
                $step!(super::execute_allocate_heap_small_shared_edge_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapZeroed => {
                $step!(super::execute_allocate_shared_heap_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapUninit => {
                $step!(super::execute_allocate_shared_heap_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapSmallZeroed => {
                $step!(super::execute_allocate_shared_heap_small_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapSmallUninit => {
                $step!(super::execute_allocate_shared_heap_small_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapZeroedBranch => {
                $transfer!(super::execute_allocate_heap_zeroed_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateHeapUninitBranch => {
                $transfer!(super::execute_allocate_heap_uninit_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapZeroedBranch => {
                $transfer!(super::execute_allocate_shared_heap_zeroed_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedHeapUninitBranch => {
                $transfer!(super::execute_allocate_shared_heap_uninit_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSliceZeroed => {
                $step!(super::execute_allocate_slice_zeroed($machine, instruction))
            }
            Op::AllocateSliceUninit => {
                $step!(super::execute_allocate_slice_uninit($machine, instruction))
            }
            Op::AllocateSharedSliceZeroed => {
                $step!(super::execute_allocate_shared_slice_zeroed(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedSliceUninit => {
                $step!(super::execute_allocate_shared_slice_uninit(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSliceZeroedBranch => {
                $transfer!(super::execute_allocate_slice_zeroed_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSliceUninitBranch => {
                $transfer!(super::execute_allocate_slice_uninit_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedSliceZeroedBranch => {
                $transfer!(super::execute_allocate_shared_slice_zeroed_branch(
                    $machine,
                    instruction
                ))
            }
            Op::AllocateSharedSliceUninitBranch => {
                $transfer!(super::execute_allocate_shared_slice_uninit_branch(
                    $machine,
                    instruction
                ))
            }
            Op::FreeHeap => $step!(super::execute_free_heap($machine, instruction)),
            Op::FreeSharedHeap => $step!(super::execute_free_shared_heap($machine, instruction)),
            Op::AllocateStackZeroed => {
                $step!(super::execute_allocate_stack_zeroed($machine, instruction))
            }
            Op::AllocateStackUninit => {
                $step!(super::execute_allocate_stack_uninit($machine, instruction))
            }
            Op::PinHeap => $step!(super::execute_pin_heap($machine, instruction)),
            Op::PinSharedHeap => $step!(super::execute_pin_shared_heap($machine, instruction)),
            Op::UnpinHeap => $step!(super::execute_unpin_heap($machine, instruction)),
            Op::UnpinSharedHeap => $step!(super::execute_unpin_shared_heap($machine, instruction)),
            Op::VectorBinary => $step!(super::execute_vector_binary($machine, instruction)),
            Op::PackedAdd32x4 => $step!(super::execute_packed_add_32x4($machine, instruction)),
            Op::PackedSub32x4 => $step!(super::execute_packed_sub_32x4($machine, instruction)),
            Op::PackedMul32x4 => $step!(super::execute_packed_mul_32x4($machine, instruction)),
            Op::PackedAnd32x4 => $step!(super::execute_packed_and_32x4($machine, instruction)),
            Op::PackedOr32x4 => $step!(super::execute_packed_or_32x4($machine, instruction)),
            Op::PackedXor32x4 => $step!(super::execute_packed_xor_32x4($machine, instruction)),
            Op::PackedShl32x4 => $step!(super::execute_packed_shl_32x4($machine, instruction)),
            Op::PackedShrI32x4 => $step!(super::execute_packed_shr_i32x4($machine, instruction)),
            Op::PackedShrU32x4 => $step!(super::execute_packed_shr_u32x4($machine, instruction)),
            Op::PackedAdd64x2 => $step!(super::execute_packed_add_64x2($machine, instruction)),
            Op::PackedSub64x2 => $step!(super::execute_packed_sub_64x2($machine, instruction)),
            Op::PackedMul64x2 => $step!(super::execute_packed_mul_64x2($machine, instruction)),
            Op::PackedAnd64x2 => $step!(super::execute_packed_and_64x2($machine, instruction)),
            Op::PackedOr64x2 => $step!(super::execute_packed_or_64x2($machine, instruction)),
            Op::PackedXor64x2 => $step!(super::execute_packed_xor_64x2($machine, instruction)),
            Op::PackedShl64x2 => $step!(super::execute_packed_shl_64x2($machine, instruction)),
            Op::PackedShrI64x2 => $step!(super::execute_packed_shr_i64x2($machine, instruction)),
            Op::PackedShrU64x2 => $step!(super::execute_packed_shr_u64x2($machine, instruction)),
            Op::PackedAddF32x4 => $step!(super::execute_packed_add_f32x4($machine, instruction)),
            Op::PackedSubF32x4 => $step!(super::execute_packed_sub_f32x4($machine, instruction)),
            Op::PackedMulF32x4 => $step!(super::execute_packed_mul_f32x4($machine, instruction)),
            Op::PackedDivF32x4 => $step!(super::execute_packed_div_f32x4($machine, instruction)),
            Op::PackedAddF64x2 => $step!(super::execute_packed_add_f64x2($machine, instruction)),
            Op::PackedSubF64x2 => $step!(super::execute_packed_sub_f64x2($machine, instruction)),
            Op::PackedMulF64x2 => $step!(super::execute_packed_mul_f64x2($machine, instruction)),
            Op::PackedDivF64x2 => $step!(super::execute_packed_div_f64x2($machine, instruction)),
            Op::TensorBinary => $step!(super::execute_tensor_binary($machine, instruction)),
            Op::TensorContiguousBinary => {
                $step!(super::execute_tensor_contiguous_binary(
                    $machine,
                    instruction
                ))
            }
            Op::AndBool => $step!(super::execute_and_bool($machine, instruction)),
            Op::OrBool => $step!(super::execute_or_bool($machine, instruction)),
            Op::XorBool => $step!(super::execute_xor_bool($machine, instruction)),
            Op::AddI32 => $step!(super::execute_add_i32($machine, instruction)),
            Op::AddU32 => $step!(super::execute_add_u32($machine, instruction)),
            Op::AddI64 => $step!(super::execute_add_i64($machine, instruction)),
            Op::AddU64 => $step!(super::execute_add_u64($machine, instruction)),
            Op::SubI32 => $step!(super::execute_sub_i32($machine, instruction)),
            Op::SubU32 => $step!(super::execute_sub_u32($machine, instruction)),
            Op::SubI64 => $step!(super::execute_sub_i64($machine, instruction)),
            Op::SubU64 => $step!(super::execute_sub_u64($machine, instruction)),
            Op::MulI32 => $step!(super::execute_mul_i32($machine, instruction)),
            Op::MulU32 => $step!(super::execute_mul_u32($machine, instruction)),
            Op::MulI64 => $step!(super::execute_mul_i64($machine, instruction)),
            Op::MulU64 => $step!(super::execute_mul_u64($machine, instruction)),
            Op::DivI32 => $step!(super::execute_div_i32($machine, instruction)),
            Op::DivU32 => $step!(super::execute_div_u32($machine, instruction)),
            Op::DivI64 => $step!(super::execute_div_i64($machine, instruction)),
            Op::DivU64 => $step!(super::execute_div_u64($machine, instruction)),
            Op::RemI32 => $step!(super::execute_rem_i32($machine, instruction)),
            Op::RemU32 => $step!(super::execute_rem_u32($machine, instruction)),
            Op::RemI64 => $step!(super::execute_rem_i64($machine, instruction)),
            Op::RemU64 => $step!(super::execute_rem_u64($machine, instruction)),
            Op::AddWordInt => $step!(super::execute_add_word_int($machine, instruction)),
            Op::AddWordUint => $step!(super::execute_add_word_uint($machine, instruction)),
            Op::SubWordInt => $step!(super::execute_sub_word_int($machine, instruction)),
            Op::SubWordUint => $step!(super::execute_sub_word_uint($machine, instruction)),
            Op::MulWordInt => $step!(super::execute_mul_word_int($machine, instruction)),
            Op::MulWordUint => $step!(super::execute_mul_word_uint($machine, instruction)),
            Op::DivWordInt => $step!(super::execute_div_word_int($machine, instruction)),
            Op::DivWordUint => $step!(super::execute_div_word_uint($machine, instruction)),
            Op::RemWordInt => $step!(super::execute_rem_word_int($machine, instruction)),
            Op::RemWordUint => $step!(super::execute_rem_word_uint($machine, instruction)),
            Op::And32 => $step!(super::execute_and_32($machine, instruction)),
            Op::And64 => $step!(super::execute_and_64($machine, instruction)),
            Op::Or32 => $step!(super::execute_or_32($machine, instruction)),
            Op::Or64 => $step!(super::execute_or_64($machine, instruction)),
            Op::Xor32 => $step!(super::execute_xor_32($machine, instruction)),
            Op::Xor64 => $step!(super::execute_xor_64($machine, instruction)),
            Op::Shl32 => $step!(super::execute_shl_32($machine, instruction)),
            Op::Shl64 => $step!(super::execute_shl_64($machine, instruction)),
            Op::ShrI32 => $step!(super::execute_shr_i32($machine, instruction)),
            Op::ShrU32 => $step!(super::execute_shr_u32($machine, instruction)),
            Op::ShrI64 => $step!(super::execute_shr_i64($machine, instruction)),
            Op::ShrU64 => $step!(super::execute_shr_u64($machine, instruction)),
            Op::AddWideInt => $step!(super::execute_add_wide_int($machine, instruction)),
            Op::SubWideInt => $step!(super::execute_sub_wide_int($machine, instruction)),
            Op::MulWideInt => $step!(super::execute_mul_wide_int($machine, instruction)),
            Op::DivWideInt => $step!(super::execute_div_wide_int($machine, instruction)),
            Op::DivWideUint => $step!(super::execute_div_wide_uint($machine, instruction)),
            Op::RemWideInt => $step!(super::execute_rem_wide_int($machine, instruction)),
            Op::RemWideUint => $step!(super::execute_rem_wide_uint($machine, instruction)),
            Op::AndWord => $step!(super::execute_and_word($machine, instruction)),
            Op::OrWord => $step!(super::execute_or_word($machine, instruction)),
            Op::XorWord => $step!(super::execute_xor_word($machine, instruction)),
            Op::ShlWord => $step!(super::execute_shl_word($machine, instruction)),
            Op::ShrWordInt => $step!(super::execute_shr_word_int($machine, instruction)),
            Op::ShrWordUint => $step!(super::execute_shr_word_uint($machine, instruction)),
            Op::AndWideInt => $step!(super::execute_and_wide_int($machine, instruction)),
            Op::OrWideInt => $step!(super::execute_or_wide_int($machine, instruction)),
            Op::XorWideInt => $step!(super::execute_xor_wide_int($machine, instruction)),
            Op::ShlWideInt => $step!(super::execute_shl_wide_int($machine, instruction)),
            Op::ShrWideInt => $step!(super::execute_shr_wide_int($machine, instruction)),
            Op::ShrWideUint => $step!(super::execute_shr_wide_uint($machine, instruction)),
            Op::AddF32 => $step!(super::execute_add_f32($machine, instruction)),
            Op::SubF32 => $step!(super::execute_sub_f32($machine, instruction)),
            Op::MulF32 => $step!(super::execute_mul_f32($machine, instruction)),
            Op::DivF32 => $step!(super::execute_div_f32($machine, instruction)),
            Op::EqF32 => $step!(super::execute_eq_f32($machine, instruction)),
            Op::NeF32 => $step!(super::execute_ne_f32($machine, instruction)),
            Op::LtF32 => $step!(super::execute_lt_f32($machine, instruction)),
            Op::LeF32 => $step!(super::execute_le_f32($machine, instruction)),
            Op::GtF32 => $step!(super::execute_gt_f32($machine, instruction)),
            Op::GeF32 => $step!(super::execute_ge_f32($machine, instruction)),
            Op::AddF64 => $step!(super::execute_add_f64($machine, instruction)),
            Op::SubF64 => $step!(super::execute_sub_f64($machine, instruction)),
            Op::MulF64 => $step!(super::execute_mul_f64($machine, instruction)),
            Op::DivF64 => $step!(super::execute_div_f64($machine, instruction)),
            Op::EqF64 => $step!(super::execute_eq_f64($machine, instruction)),
            Op::NeF64 => $step!(super::execute_ne_f64($machine, instruction)),
            Op::LtF64 => $step!(super::execute_lt_f64($machine, instruction)),
            Op::LeF64 => $step!(super::execute_le_f64($machine, instruction)),
            Op::GtF64 => $step!(super::execute_gt_f64($machine, instruction)),
            Op::GeF64 => $step!(super::execute_ge_f64($machine, instruction)),
            Op::Eq32 => $step!(super::execute_eq_32($machine, instruction)),
            Op::Eq64 => $step!(super::execute_eq_64($machine, instruction)),
            Op::Ne32 => $step!(super::execute_ne_32($machine, instruction)),
            Op::Ne64 => $step!(super::execute_ne_64($machine, instruction)),
            Op::LtI32 => $step!(super::execute_lt_i32($machine, instruction)),
            Op::LtU32 => $step!(super::execute_lt_u32($machine, instruction)),
            Op::LtI64 => $step!(super::execute_lt_i64($machine, instruction)),
            Op::LtU64 => $step!(super::execute_lt_u64($machine, instruction)),
            Op::LeI32 => $step!(super::execute_le_i32($machine, instruction)),
            Op::LeU32 => $step!(super::execute_le_u32($machine, instruction)),
            Op::LeI64 => $step!(super::execute_le_i64($machine, instruction)),
            Op::LeU64 => $step!(super::execute_le_u64($machine, instruction)),
            Op::GtI32 => $step!(super::execute_gt_i32($machine, instruction)),
            Op::GtU32 => $step!(super::execute_gt_u32($machine, instruction)),
            Op::GtI64 => $step!(super::execute_gt_i64($machine, instruction)),
            Op::GtU64 => $step!(super::execute_gt_u64($machine, instruction)),
            Op::GeI32 => $step!(super::execute_ge_i32($machine, instruction)),
            Op::GeU32 => $step!(super::execute_ge_u32($machine, instruction)),
            Op::GeI64 => $step!(super::execute_ge_i64($machine, instruction)),
            Op::GeU64 => $step!(super::execute_ge_u64($machine, instruction)),
            Op::EqWord => $step!(super::execute_eq_word($machine, instruction)),
            Op::NeWord => $step!(super::execute_ne_word($machine, instruction)),
            Op::LtWordInt => $step!(super::execute_lt_word_int($machine, instruction)),
            Op::LtWordUint => $step!(super::execute_lt_word_uint($machine, instruction)),
            Op::LeWordInt => $step!(super::execute_le_word_int($machine, instruction)),
            Op::LeWordUint => $step!(super::execute_le_word_uint($machine, instruction)),
            Op::GtWordInt => $step!(super::execute_gt_word_int($machine, instruction)),
            Op::GtWordUint => $step!(super::execute_gt_word_uint($machine, instruction)),
            Op::GeWordInt => $step!(super::execute_ge_word_int($machine, instruction)),
            Op::GeWordUint => $step!(super::execute_ge_word_uint($machine, instruction)),
            Op::EqWideInt => $step!(super::execute_eq_wide_int($machine, instruction)),
            Op::NeWideInt => $step!(super::execute_ne_wide_int($machine, instruction)),
            Op::LtWideInt => $step!(super::execute_lt_wide_int($machine, instruction)),
            Op::LtWideUint => $step!(super::execute_lt_wide_uint($machine, instruction)),
            Op::LeWideInt => $step!(super::execute_le_wide_int($machine, instruction)),
            Op::LeWideUint => $step!(super::execute_le_wide_uint($machine, instruction)),
            Op::GtWideInt => $step!(super::execute_gt_wide_int($machine, instruction)),
            Op::GtWideUint => $step!(super::execute_gt_wide_uint($machine, instruction)),
            Op::GeWideInt => $step!(super::execute_ge_wide_int($machine, instruction)),
            Op::GeWideUint => $step!(super::execute_ge_wide_uint($machine, instruction)),
            Op::NegI32 => $step!(super::execute_neg_i32($machine, instruction)),
            Op::NegI64 => $step!(super::execute_neg_i64($machine, instruction)),
            Op::Not32 => $step!(super::execute_not_32($machine, instruction)),
            Op::Not64 => $step!(super::execute_not_64($machine, instruction)),
            Op::NegWordInt => $step!(super::execute_neg_word_int($machine, instruction)),
            Op::NotWord => $step!(super::execute_not_word($machine, instruction)),
            Op::NegWideInt => $step!(super::execute_neg_wide_int($machine, instruction)),
            Op::NotWideInt => $step!(super::execute_not_wide_int($machine, instruction)),
            Op::NegF32 => $step!(super::execute_neg_f32($machine, instruction)),
            Op::NegF64 => $step!(super::execute_neg_f64($machine, instruction)),
            Op::NotBool => $step!(super::execute_not_bool($machine, instruction)),
            Op::VectorUnary => $step!(super::execute_vector_unary($machine, instruction)),
            Op::TensorUnary => $step!(super::execute_tensor_unary($machine, instruction)),
            Op::TensorContiguousUnary => {
                $step!(super::execute_tensor_contiguous_unary(
                    $machine,
                    instruction
                ))
            }
            Op::PackedNegI32x4 => $step!(super::execute_packed_neg_i32x4($machine, instruction)),
            Op::PackedNot32x4 => $step!(super::execute_packed_not_32x4($machine, instruction)),
            Op::PackedNegI64x2 => $step!(super::execute_packed_neg_i64x2($machine, instruction)),
            Op::PackedNot64x2 => $step!(super::execute_packed_not_64x2($machine, instruction)),
            Op::PackedNegF32x4 => $step!(super::execute_packed_neg_f32x4($machine, instruction)),
            Op::PackedNegF64x2 => $step!(super::execute_packed_neg_f64x2($machine, instruction)),
            Op::CastBitcast => $step!(super::execute_cast_bitcast($machine, instruction)),
            Op::CastTruncate => $step!(super::execute_cast_truncate($machine, instruction)),
            Op::CastZeroExtend => $step!(super::execute_cast_zero_extend($machine, instruction)),
            Op::CastSignExtend => $step!(super::execute_cast_sign_extend($machine, instruction)),
            Op::CastFloatToSignedInt => $step!(super::execute_cast_float_to_signed_int(
                $machine,
                instruction
            )),
            Op::CastFloatToUnsignedInt => {
                $step!({ super::execute_cast_float_to_unsigned_int($machine, instruction) })
            }
            Op::CastFloatToSignedIntSaturating => {
                $step!({
                    super::execute_cast_float_to_signed_int_saturating($machine, instruction)
                })
            }
            Op::CastFloatToUnsignedIntSaturating => $step!({
                super::execute_cast_float_to_unsigned_int_saturating($machine, instruction)
            }),
            Op::CastSignedIntToF32 => {
                $step!(super::execute_cast_signed_int_to_f32($machine, instruction))
            }
            Op::CastSignedIntToF64 => {
                $step!(super::execute_cast_signed_int_to_f64($machine, instruction))
            }
            Op::CastUnsignedIntToF32 => $step!(super::execute_cast_unsigned_int_to_f32(
                $machine,
                instruction
            )),
            Op::CastUnsignedIntToF64 => $step!(super::execute_cast_unsigned_int_to_f64(
                $machine,
                instruction
            )),
            Op::CastFloatTruncate => {
                $step!(super::execute_cast_float_truncate($machine, instruction))
            }
            Op::CastFloatExtend => $step!(super::execute_cast_float_extend($machine, instruction)),
            Op::CastPointerToInt => {
                $step!(super::execute_cast_pointer_to_int($machine, instruction))
            }
            Op::CastIntToPointer => {
                $step!(super::execute_cast_int_to_pointer($machine, instruction))
            }
            Op::CastWordToWideInt => {
                $step!(super::execute_cast_word_to_wide_int($machine, instruction))
            }
            Op::CastWideIntToWord => {
                $step!(super::execute_cast_wide_int_to_word($machine, instruction))
            }
            Op::CastWideInt => $step!(super::execute_cast_wide_int($machine, instruction)),
            Op::CastTensorView => $step!(super::execute_tensor_view_cast($machine, instruction)),
            Op::Call => $transfer!(super::execute_call($machine, instruction, $block_pc)),
            Op::CallBranch => $transfer!(super::execute_call_branch($machine, instruction)),
            Op::CallIndirect => {
                $transfer!(super::execute_call_indirect(
                    $machine,
                    instruction,
                    $block_pc
                ))
            }
            Op::CallCallable => {
                $transfer!(super::execute_call_callable(
                    $machine,
                    instruction,
                    $block_pc
                ))
            }
            Op::CallIndirectBranch => {
                $transfer!(super::execute_call_indirect_branch($machine, instruction))
            }
            Op::CallCallableBranch => {
                $transfer!(super::execute_call_callable_branch($machine, instruction))
            }
            Op::CallClassHeap => $transfer!(super::execute_call_class_heap(
                $machine,
                instruction,
                $block_pc
            )),
            Op::CallClassSharedHeap => $transfer!({
                super::execute_call_class_shared_heap($machine, instruction, $block_pc)
            }),
            Op::CallClassHeapBranch => {
                $transfer!(super::execute_call_class_heap_branch($machine, instruction))
            }
            Op::CallClassSharedHeapBranch => {
                $transfer!(super::execute_call_class_shared_heap_branch(
                    $machine,
                    instruction
                ))
            }
            Op::CallInterfaceHeap => $transfer!(super::execute_call_interface_heap(
                $machine,
                instruction,
                $block_pc
            )),
            Op::CallInterfaceSharedHeap => $transfer!({
                super::execute_call_interface_shared_heap($machine, instruction, $block_pc)
            }),
            Op::CallInterfaceHeapBranch => {
                $transfer!(super::execute_call_interface_heap_branch(
                    $machine,
                    instruction
                ))
            }
            Op::CallInterfaceSharedHeapBranch => {
                $transfer!(super::execute_call_interface_shared_heap_branch(
                    $machine,
                    instruction
                ))
            }
            Op::TailCall => $transfer!(super::execute_tail_call($machine, instruction)),
            Op::TailCallSelf => $transfer!(super::execute_tail_call_self($machine, instruction)),
            Op::TailCallIndirect => {
                $transfer!(super::execute_tail_call_indirect($machine, instruction))
            }
            Op::TailCallCallable => {
                $transfer!(super::execute_tail_call_callable($machine, instruction))
            }
            Op::TailCallClassHeap => {
                $transfer!(super::execute_tail_call_class_heap($machine, instruction))
            }
            Op::TailCallClassSharedHeap => {
                $transfer!(super::execute_tail_call_class_shared_heap(
                    $machine,
                    instruction
                ))
            }
            Op::TailCallInterfaceHeap => {
                $transfer!(super::execute_tail_call_interface_heap(
                    $machine,
                    instruction
                ))
            }
            Op::TailCallInterfaceSharedHeap => {
                $transfer!(super::execute_tail_call_interface_shared_heap(
                    $machine,
                    instruction
                ))
            }
            Op::Jump => $transfer!(super::execute_jump($machine, instruction)),
            Op::BranchBool => $transfer!(super::execute_branch_bool($machine, instruction)),
            Op::BranchEq32 => $transfer!(super::execute_branch_eq_32($machine, instruction)),
            Op::BranchEq64 => $transfer!(super::execute_branch_eq_64($machine, instruction)),
            Op::BranchNe32 => $transfer!(super::execute_branch_ne_32($machine, instruction)),
            Op::BranchNe64 => $transfer!(super::execute_branch_ne_64($machine, instruction)),
            Op::BranchLtI32 => $transfer!(super::execute_branch_lt_i32($machine, instruction)),
            Op::BranchLtU32 => $transfer!(super::execute_branch_lt_u32($machine, instruction)),
            Op::BranchLtI64 => $transfer!(super::execute_branch_lt_i64($machine, instruction)),
            Op::BranchLtU64 => $transfer!(super::execute_branch_lt_u64($machine, instruction)),
            Op::BranchLeI32 => $transfer!(super::execute_branch_le_i32($machine, instruction)),
            Op::BranchLeU32 => $transfer!(super::execute_branch_le_u32($machine, instruction)),
            Op::BranchLeI64 => $transfer!(super::execute_branch_le_i64($machine, instruction)),
            Op::BranchLeU64 => $transfer!(super::execute_branch_le_u64($machine, instruction)),
            Op::BranchGtI32 => $transfer!(super::execute_branch_gt_i32($machine, instruction)),
            Op::BranchGtU32 => $transfer!(super::execute_branch_gt_u32($machine, instruction)),
            Op::BranchGtI64 => $transfer!(super::execute_branch_gt_i64($machine, instruction)),
            Op::BranchGtU64 => $transfer!(super::execute_branch_gt_u64($machine, instruction)),
            Op::BranchGeI32 => $transfer!(super::execute_branch_ge_i32($machine, instruction)),
            Op::BranchGeU32 => $transfer!(super::execute_branch_ge_u32($machine, instruction)),
            Op::BranchGeI64 => $transfer!(super::execute_branch_ge_i64($machine, instruction)),
            Op::BranchGeU64 => $transfer!(super::execute_branch_ge_u64($machine, instruction)),
            Op::BranchEqWord => $transfer!(super::execute_branch_eq_word($machine, instruction)),
            Op::BranchNeWord => $transfer!(super::execute_branch_ne_word($machine, instruction)),
            Op::BranchLtWordInt => {
                $transfer!(super::execute_branch_lt_word_int($machine, instruction))
            }
            Op::BranchLeWordInt => {
                $transfer!(super::execute_branch_le_word_int($machine, instruction))
            }
            Op::BranchGtWordInt => {
                $transfer!(super::execute_branch_gt_word_int($machine, instruction))
            }
            Op::BranchGeWordInt => {
                $transfer!(super::execute_branch_ge_word_int($machine, instruction))
            }
            Op::BranchLtWordUint => {
                $transfer!(super::execute_branch_lt_word_uint($machine, instruction))
            }
            Op::BranchLeWordUint => {
                $transfer!(super::execute_branch_le_word_uint($machine, instruction))
            }
            Op::BranchGtWordUint => {
                $transfer!(super::execute_branch_gt_word_uint($machine, instruction))
            }
            Op::BranchGeWordUint => {
                $transfer!(super::execute_branch_ge_word_uint($machine, instruction))
            }
            Op::BranchEqF32 => $transfer!(super::execute_branch_eq_f32($machine, instruction)),
            Op::BranchNeF32 => $transfer!(super::execute_branch_ne_f32($machine, instruction)),
            Op::BranchLtF32 => $transfer!(super::execute_branch_lt_f32($machine, instruction)),
            Op::BranchLeF32 => $transfer!(super::execute_branch_le_f32($machine, instruction)),
            Op::BranchGtF32 => $transfer!(super::execute_branch_gt_f32($machine, instruction)),
            Op::BranchGeF32 => $transfer!(super::execute_branch_ge_f32($machine, instruction)),
            Op::BranchEqF64 => $transfer!(super::execute_branch_eq_f64($machine, instruction)),
            Op::BranchNeF64 => $transfer!(super::execute_branch_ne_f64($machine, instruction)),
            Op::BranchLtF64 => $transfer!(super::execute_branch_lt_f64($machine, instruction)),
            Op::BranchLeF64 => $transfer!(super::execute_branch_le_f64($machine, instruction)),
            Op::BranchGtF64 => $transfer!(super::execute_branch_gt_f64($machine, instruction)),
            Op::BranchGeF64 => $transfer!(super::execute_branch_ge_f64($machine, instruction)),
            Op::Switch => $transfer!(super::execute_switch($machine, instruction)),
            Op::SwitchTable => $transfer!(super::execute_switch_table($machine, instruction)),
            Op::Check => $transfer!(super::execute_check($machine, instruction)),
            Op::Assume => $step!(super::execute_assume($machine, instruction)),
            Op::BarrierWriteHeap => {
                $step!(super::execute_barrier_write_heap($machine, instruction))
            }
            Op::BarrierWriteSharedHeap => {
                $step!(super::execute_barrier_write_shared_heap(
                    $machine,
                    instruction
                ))
            }
            Op::AtomicLoad => $step!(super::execute_atomic_load($machine, instruction)),
            Op::AtomicStore => $step!(super::execute_atomic_store($machine, instruction)),
            Op::AtomicExchange => $step!(super::execute_atomic_exchange($machine, instruction)),
            Op::AtomicCompareExchange => {
                $step!(super::execute_atomic_compare_exchange(
                    $machine,
                    instruction
                ))
            }
            Op::AtomicReadModifyWrite => {
                $step!(super::execute_atomic_read_modify_write(
                    $machine,
                    instruction
                ))
            }
            Op::AtomicFence => $step!(super::execute_atomic_fence($machine, instruction)),
            Op::Intrinsic => $step!(super::execute_intrinsic($machine, instruction)),
            Op::ReturnWord => $transfer!(super::execute_return_word($machine, instruction)),
            Op::ReturnAddress => $transfer!(super::execute_return_address($machine, instruction)),
            Op::ReturnVoid => $transfer!(super::execute_return_void($machine, instruction)),
            Op::TensorBroadcast => $step!(super::execute_tensor_broadcast($machine, instruction)),
            Op::TensorCast => $step!(super::execute_tensor_cast($machine, instruction)),
            Op::TensorConcat => $step!(super::execute_tensor_concat($machine, instruction)),
            Op::TensorConvert => $step!(super::execute_tensor_convert($machine, instruction)),
            Op::TensorConvolution => {
                $step!(super::execute_tensor_convolution($machine, instruction))
            }
            Op::TensorCopy => $step!(super::execute_tensor_copy($machine, instruction)),
            Op::TensorDot => $step!(super::execute_tensor_dot($machine, instruction)),
            Op::TensorFill => $step!(super::execute_tensor_fill($machine, instruction)),
            Op::TensorGather => $step!(super::execute_tensor_gather($machine, instruction)),
            Op::TensorExtract => $step!(super::execute_tensor_extract($machine, instruction)),
            Op::TensorIndexReduce => {
                $step!(super::execute_tensor_index_reduce($machine, instruction))
            }
            Op::TensorLoad => $step!(super::execute_tensor_load($machine, instruction)),
            Op::TensorSplat => $step!(super::execute_tensor_splat($machine, instruction)),
            Op::TensorPad => $step!(super::execute_tensor_pad($machine, instruction)),
            Op::TensorReduce => $step!(super::execute_tensor_reduce($machine, instruction)),
            Op::TensorReshape => $step!(super::execute_tensor_reshape($machine, instruction)),
            Op::TensorScatter => $step!(super::execute_tensor_scatter($machine, instruction)),
            Op::TensorSelect => $step!(super::execute_tensor_select($machine, instruction)),
            Op::TensorSlice => $step!(super::execute_tensor_slice($machine, instruction)),
            Op::TensorStore => $step!(super::execute_tensor_store($machine, instruction)),
            Op::TensorTranspose => $step!(super::execute_tensor_transpose($machine, instruction)),
            Op::TensorView => $step!(super::execute_tensor_view($machine, instruction)),
            Op::Panic | Op::PanicValue | Op::ResumePanic => {
                $transfer!(super::execute_panic($machine, instruction))
            }
            Op::Unreachable => $transfer!(super::execute_unreachable($machine, instruction)),
            Op::VectorConvert => $step!(super::execute_vector_convert($machine, instruction)),
            Op::VectorExtract => $step!(super::execute_vector_extract($machine, instruction)),
            Op::VectorInsert => $step!(super::execute_vector_insert($machine, instruction)),
            Op::VectorReduce => $step!(super::execute_vector_reduce($machine, instruction)),
            Op::VectorSelect => $step!(super::execute_vector_select($machine, instruction)),
            Op::VectorShuffle => $step!(super::execute_vector_shuffle($machine, instruction)),
            Op::VectorSplat => $step!(super::execute_vector_splat($machine, instruction)),
            Op::PackedSplat32x4 => $step!(super::execute_packed_splat_32x4($machine, instruction)),
            Op::PackedSplat64x2 => $step!(super::execute_packed_splat_64x2($machine, instruction)),
            Op::YieldWord => $transfer!(super::execute_yield_word($machine, instruction)),
            Op::YieldAddress => $transfer!(super::execute_yield_address($machine, instruction)),
        }
    };
}

/// Dispatch one block until it produces a control transfer.
pub(crate) fn dispatch_block(
    machine: &mut Machine<'_, '_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> Transfer {
    dispatch_block_inner(machine, function, block_index, pc)
}

/// Dispatch one block in the trusted lowered-code interpreter.
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
fn dispatch_block_inner(
    machine: &mut Machine<'_, '_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> Transfer {
    let program = machine.program;
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
                            match enter_block(machine, function, target, moves) {
                                Ok((block_start, block_end)) => {
                                    (block_start, block_start, block_end)
                                }
                                Err(error) => return Transfer::Error(error),
                            };
                        continue;
                    }
                    Transfer::Enter => {
                        let frame = machine.active_frame();
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

        dispatch_instruction!(machine, function, pc, pc - block_start, step, transfer);
    }
}

/// Dispatch one block and count executed instructions.
pub(crate) fn dispatch_block_counted(
    machine: &mut Machine<'_, '_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> BlockDispatch {
    dispatch_block_counted_inner(machine, function, block_index, pc)
}

/// Dispatch one counted block in the trusted lowered-code interpreter.
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
fn dispatch_block_counted_inner(
    machine: &mut Machine<'_, '_>,
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

        dispatch_instruction!(machine, function, pc, pc - block_start, step, transfer);
    }
}
