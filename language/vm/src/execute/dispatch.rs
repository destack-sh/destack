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
        .ok_or(Error::InvalidInstruction)?;
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
    let program = machine.program;
    let move_pool = function.move_pool.as_slice();

    // move block parameters before retargeting the frame
    let frame = machine.current_frame_mut();
    move_values_within_frame(program, frame, moves, move_pool)?;

    // retarget the frame to the destination block
    let frame = machine.current_frame_mut();
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

/// Dispatch one block until it produces a control transfer.
#[inline(always)]
pub(crate) fn dispatch_block(
    machine: &mut Machine<'_, '_>,
    function: &Function,
    block_index: u32,
    pc: usize,
) -> Transfer {
    let mut function_ptr = function as *const Function;

    // start at the requested block offset
    let (mut block_start, mut pc, mut block_end) =
        match block_bounds(unsafe { &*function_ptr }, block_index) {
            Ok((block_start, block_end)) => (block_start, block_start + pc, block_end),
            Err(error) => return Transfer::Error(error),
        };

    loop {
        // guard against malformed block metadata
        if pc >= block_end {
            return Transfer::Error(Error::InvalidInstruction);
        }

        match dispatch_op(machine, unsafe { &*function_ptr }, pc, pc - block_start) {
            Transfer::Continue => {
                pc += 1;
            }
            Transfer::Jump {
                block: target,
                moves,
            } => {
                let function = unsafe { &*function_ptr };
                (block_start, pc, block_end) = match enter_block(machine, function, target, moves) {
                    Ok((block_start, block_end)) => (block_start, block_start, block_end),
                    Err(error) => return Transfer::Error(error),
                };
            }
            Transfer::Enter => {
                let frame = machine.current_frame_mut();
                function_ptr = frame.function_ptr.as_ptr();
                let function_ref = unsafe { frame.function_ptr.as_ref() };
                (block_start, pc, block_end) = match block_bounds(function_ref, frame.block) {
                    Ok((block_start, block_end)) => (block_start, block_start, block_end),
                    Err(error) => return Transfer::Error(error),
                };
            }
            transfer => return transfer,
        }
    }
}

/// Dispatch one block and count executed instructions.
pub(crate) fn dispatch_block_counted(
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
                transfer: Transfer::Error(Error::InvalidInstruction),
                executed,
            };
        }

        executed += 1;
        match dispatch_op(machine, function, pc, pc - block_start) {
            Transfer::Continue => {
                pc += 1;
            }
            transfer => return BlockDispatch { transfer, executed },
        }
    }
}

/// Dispatch one program operation.
#[inline(always)]
fn dispatch_op(
    machine: &mut Machine<'_, '_>,
    function: &Function,
    pc: usize,
    block_pc: usize,
) -> Transfer {
    let instruction = &function.code[pc];

    match instruction.op {
        Op::LoadConstWord => super::execute_load_const_word(machine, instruction),
        Op::LoadConstBytes => super::execute_load_const_bytes(machine, instruction),
        Op::MoveFrame => super::execute_move_frame(machine, instruction),
        Op::LoadHeapBytes => super::execute_load_heap_bytes(machine, instruction),
        Op::LoadSharedHeapBytes => super::execute_load_shared_heap_bytes(machine, instruction),
        Op::LoadRawBytes => super::execute_load_raw_bytes(machine, instruction),
        Op::LoadSharedRawBytes => super::execute_load_shared_raw_bytes(machine, instruction),
        Op::LoadStackBytes => super::execute_load_stack_bytes(machine, instruction),
        Op::LoadFrameBytes => super::execute_load_frame_bytes(machine, instruction),
        Op::LoadStaticBytes => super::execute_load_static_bytes(machine, instruction),
        Op::StoreHeapBytes => super::execute_store_heap_bytes(machine, instruction),
        Op::StoreSharedHeapBytes => super::execute_store_shared_heap_bytes(machine, instruction),
        Op::StoreRawBytes => super::execute_store_raw_bytes(machine, instruction),
        Op::StoreSharedRawBytes => super::execute_store_shared_raw_bytes(machine, instruction),
        Op::StoreStackBytes => super::execute_store_stack_bytes(machine, instruction),
        Op::StoreFrameBytes => super::execute_store_frame_bytes(machine, instruction),
        Op::StoreStaticBytes => super::execute_store_static_bytes(machine, instruction),
        Op::SelectWord => super::execute_select_word(machine, instruction),
        Op::SelectFrame => super::execute_select_frame(machine, instruction),
        Op::LoadLocal => super::execute_load_local(machine, instruction),
        Op::StoreLocal => super::execute_store_local(machine, instruction),
        Op::AddressLocal => super::execute_address_local(machine, instruction),
        Op::AddressStatic => super::execute_address_static(machine, instruction),
        Op::AddressFunction => super::execute_address_function(machine, instruction),
        Op::BindCallableWord => super::execute_bind_callable_word(machine, instruction),
        Op::BindCallableAddress => super::execute_bind_callable_address(machine, instruction),
        Op::LoadCallableEnvironment => {
            super::execute_load_callable_environment(machine, instruction)
        }
        Op::LoadHeapU8 => super::execute_load_heap_scalar::<1, false>(machine, instruction),
        Op::LoadHeapI8 => super::execute_load_heap_scalar::<1, true>(machine, instruction),
        Op::LoadHeapU16 => super::execute_load_heap_scalar::<2, false>(machine, instruction),
        Op::LoadHeapI16 => super::execute_load_heap_scalar::<2, true>(machine, instruction),
        Op::LoadHeapU32 => super::execute_load_heap_scalar::<4, false>(machine, instruction),
        Op::LoadHeapI32 => super::execute_load_heap_scalar::<4, true>(machine, instruction),
        Op::LoadHeap64 => super::execute_load_heap_scalar::<8, false>(machine, instruction),
        Op::LoadSharedHeapU8 => {
            super::execute_load_shared_heap_scalar::<1, false>(machine, instruction)
        }
        Op::LoadSharedHeapI8 => {
            super::execute_load_shared_heap_scalar::<1, true>(machine, instruction)
        }
        Op::LoadSharedHeapU16 => {
            super::execute_load_shared_heap_scalar::<2, false>(machine, instruction)
        }
        Op::LoadSharedHeapI16 => {
            super::execute_load_shared_heap_scalar::<2, true>(machine, instruction)
        }
        Op::LoadSharedHeapU32 => {
            super::execute_load_shared_heap_scalar::<4, false>(machine, instruction)
        }
        Op::LoadSharedHeapI32 => {
            super::execute_load_shared_heap_scalar::<4, true>(machine, instruction)
        }
        Op::LoadSharedHeap64 => {
            super::execute_load_shared_heap_scalar::<8, false>(machine, instruction)
        }
        Op::LoadRawU8 => super::execute_load_raw_scalar::<1, false>(machine, instruction),
        Op::LoadRawI8 => super::execute_load_raw_scalar::<1, true>(machine, instruction),
        Op::LoadRawU16 => super::execute_load_raw_scalar::<2, false>(machine, instruction),
        Op::LoadRawI16 => super::execute_load_raw_scalar::<2, true>(machine, instruction),
        Op::LoadRawU32 => super::execute_load_raw_scalar::<4, false>(machine, instruction),
        Op::LoadRawI32 => super::execute_load_raw_scalar::<4, true>(machine, instruction),
        Op::LoadRaw64 => super::execute_load_raw_scalar::<8, false>(machine, instruction),
        Op::LoadSharedRawU8 => {
            super::execute_load_shared_raw_scalar::<1, false>(machine, instruction)
        }
        Op::LoadSharedRawI8 => {
            super::execute_load_shared_raw_scalar::<1, true>(machine, instruction)
        }
        Op::LoadSharedRawU16 => {
            super::execute_load_shared_raw_scalar::<2, false>(machine, instruction)
        }
        Op::LoadSharedRawI16 => {
            super::execute_load_shared_raw_scalar::<2, true>(machine, instruction)
        }
        Op::LoadSharedRawU32 => {
            super::execute_load_shared_raw_scalar::<4, false>(machine, instruction)
        }
        Op::LoadSharedRawI32 => {
            super::execute_load_shared_raw_scalar::<4, true>(machine, instruction)
        }
        Op::LoadSharedRaw64 => {
            super::execute_load_shared_raw_scalar::<8, false>(machine, instruction)
        }
        Op::LoadStackU8 => super::execute_load_stack_scalar::<1, false>(machine, instruction),
        Op::LoadStackI8 => super::execute_load_stack_scalar::<1, true>(machine, instruction),
        Op::LoadStackU16 => super::execute_load_stack_scalar::<2, false>(machine, instruction),
        Op::LoadStackI16 => super::execute_load_stack_scalar::<2, true>(machine, instruction),
        Op::LoadStackU32 => super::execute_load_stack_scalar::<4, false>(machine, instruction),
        Op::LoadStackI32 => super::execute_load_stack_scalar::<4, true>(machine, instruction),
        Op::LoadStack64 => super::execute_load_stack_scalar::<8, false>(machine, instruction),
        Op::LoadFrameU8 => super::execute_load_frame_scalar::<1, false>(machine, instruction),
        Op::LoadFrameI8 => super::execute_load_frame_scalar::<1, true>(machine, instruction),
        Op::LoadFrameU16 => super::execute_load_frame_scalar::<2, false>(machine, instruction),
        Op::LoadFrameI16 => super::execute_load_frame_scalar::<2, true>(machine, instruction),
        Op::LoadFrameU32 => super::execute_load_frame_scalar::<4, false>(machine, instruction),
        Op::LoadFrameI32 => super::execute_load_frame_scalar::<4, true>(machine, instruction),
        Op::LoadFrame64 => super::execute_load_frame_scalar::<8, false>(machine, instruction),
        Op::LoadFrameValueU8 => {
            super::execute_load_frame_value_scalar::<1, false>(machine, instruction)
        }
        Op::LoadFrameValueI8 => {
            super::execute_load_frame_value_scalar::<1, true>(machine, instruction)
        }
        Op::LoadFrameValueU16 => {
            super::execute_load_frame_value_scalar::<2, false>(machine, instruction)
        }
        Op::LoadFrameValueI16 => {
            super::execute_load_frame_value_scalar::<2, true>(machine, instruction)
        }
        Op::LoadFrameValueU32 => {
            super::execute_load_frame_value_scalar::<4, false>(machine, instruction)
        }
        Op::LoadFrameValueI32 => {
            super::execute_load_frame_value_scalar::<4, true>(machine, instruction)
        }
        Op::LoadFrameValue64 => {
            super::execute_load_frame_value_scalar::<8, false>(machine, instruction)
        }
        Op::LoadStaticU8 => super::execute_load_static_scalar::<1, false>(machine, instruction),
        Op::LoadStaticI8 => super::execute_load_static_scalar::<1, true>(machine, instruction),
        Op::LoadStaticU16 => super::execute_load_static_scalar::<2, false>(machine, instruction),
        Op::LoadStaticI16 => super::execute_load_static_scalar::<2, true>(machine, instruction),
        Op::LoadStaticU32 => super::execute_load_static_scalar::<4, false>(machine, instruction),
        Op::LoadStaticI32 => super::execute_load_static_scalar::<4, true>(machine, instruction),
        Op::LoadStatic64 => super::execute_load_static_scalar::<8, false>(machine, instruction),
        Op::StoreHeap8 => super::execute_store_heap_scalar::<1>(machine, instruction),
        Op::StoreHeap16 => super::execute_store_heap_scalar::<2>(machine, instruction),
        Op::StoreHeap32 => super::execute_store_heap_scalar::<4>(machine, instruction),
        Op::StoreHeap64 => super::execute_store_heap_scalar::<8>(machine, instruction),
        Op::StoreSharedHeap8 => super::execute_store_shared_heap_scalar::<1>(machine, instruction),
        Op::StoreSharedHeap16 => super::execute_store_shared_heap_scalar::<2>(machine, instruction),
        Op::StoreSharedHeap32 => super::execute_store_shared_heap_scalar::<4>(machine, instruction),
        Op::StoreSharedHeap64 => super::execute_store_shared_heap_scalar::<8>(machine, instruction),
        Op::StoreRaw8 => super::execute_store_raw_scalar::<1>(machine, instruction),
        Op::StoreRaw16 => super::execute_store_raw_scalar::<2>(machine, instruction),
        Op::StoreRaw32 => super::execute_store_raw_scalar::<4>(machine, instruction),
        Op::StoreRaw64 => super::execute_store_raw_scalar::<8>(machine, instruction),
        Op::StoreSharedRaw8 => super::execute_store_shared_raw_scalar::<1>(machine, instruction),
        Op::StoreSharedRaw16 => super::execute_store_shared_raw_scalar::<2>(machine, instruction),
        Op::StoreSharedRaw32 => super::execute_store_shared_raw_scalar::<4>(machine, instruction),
        Op::StoreSharedRaw64 => super::execute_store_shared_raw_scalar::<8>(machine, instruction),
        Op::StoreStack8 => super::execute_store_stack_scalar::<1>(machine, instruction),
        Op::StoreStack16 => super::execute_store_stack_scalar::<2>(machine, instruction),
        Op::StoreStack32 => super::execute_store_stack_scalar::<4>(machine, instruction),
        Op::StoreStack64 => super::execute_store_stack_scalar::<8>(machine, instruction),
        Op::StoreFrame8 => super::execute_store_frame_scalar::<1>(machine, instruction),
        Op::StoreFrame16 => super::execute_store_frame_scalar::<2>(machine, instruction),
        Op::StoreFrame32 => super::execute_store_frame_scalar::<4>(machine, instruction),
        Op::StoreFrame64 => super::execute_store_frame_scalar::<8>(machine, instruction),
        Op::StoreFrameValue8 => super::execute_store_frame_value_scalar::<1>(machine, instruction),
        Op::StoreFrameValue16 => super::execute_store_frame_value_scalar::<2>(machine, instruction),
        Op::StoreFrameValue32 => super::execute_store_frame_value_scalar::<4>(machine, instruction),
        Op::StoreFrameValue64 => super::execute_store_frame_value_scalar::<8>(machine, instruction),
        Op::StoreStatic8 => super::execute_store_static_scalar::<1>(machine, instruction),
        Op::StoreStatic16 => super::execute_store_static_scalar::<2>(machine, instruction),
        Op::StoreStatic32 => super::execute_store_static_scalar::<4>(machine, instruction),
        Op::StoreStatic64 => super::execute_store_static_scalar::<8>(machine, instruction),
        Op::Abort => super::execute_abort(machine, instruction),
        Op::AddressFrameOffset => super::execute_address_frame_offset(machine, instruction),
        Op::AddressFrameElement => super::execute_address_frame_element(machine, instruction),
        Op::AddressHeapOffset => super::execute_address_heap_offset(machine, instruction),
        Op::AddressSharedHeapOffset => {
            super::execute_address_shared_heap_offset(machine, instruction)
        }
        Op::AddressRawOffset => super::execute_address_raw_offset(machine, instruction),
        Op::AddressSharedRawOffset => {
            super::execute_address_shared_raw_offset(machine, instruction)
        }
        Op::AddressStackOffset => super::execute_address_stack_offset(machine, instruction),
        Op::AddressStaticOffset => super::execute_address_static_offset(machine, instruction),
        Op::AddressHeapElement => super::execute_address_heap_element(machine, instruction),
        Op::AddressSharedHeapElement => {
            super::execute_address_shared_heap_element(machine, instruction)
        }
        Op::AddressRawElement => super::execute_address_raw_element(machine, instruction),
        Op::AddressSharedRawElement => {
            super::execute_address_shared_raw_element(machine, instruction)
        }
        Op::AddressStackElement => super::execute_address_stack_element(machine, instruction),
        Op::AddressStaticElement => super::execute_address_static_element(machine, instruction),
        Op::AddressHeapSliceElement => {
            super::execute_address_heap_slice_element(machine, instruction)
        }
        Op::AddressSharedHeapSliceElement => {
            super::execute_address_shared_heap_slice_element(machine, instruction)
        }
        Op::AddressRawSliceElement => {
            super::execute_address_raw_slice_element(machine, instruction)
        }
        Op::AddressSharedRawSliceElement => {
            super::execute_address_shared_raw_slice_element(machine, instruction)
        }
        Op::AddressStackSliceElement => {
            super::execute_address_stack_slice_element(machine, instruction)
        }
        Op::AddressFrameSliceElement => {
            super::execute_address_frame_slice_element(machine, instruction)
        }
        Op::AddressStaticSliceElement => {
            super::execute_address_static_slice_element(machine, instruction)
        }
        Op::AllocateHeapSmallNoscan => {
            super::execute_allocate_heap_small_noscan(machine, instruction)
        }
        Op::AllocateHeap => super::execute_allocate_heap(machine, instruction),
        Op::AllocateSharedHeapSmallNoscan => {
            super::execute_allocate_shared_heap_small_noscan(machine, instruction)
        }
        Op::AllocateSharedHeap => super::execute_allocate_shared_heap(machine, instruction),
        Op::AllocateSlice => super::execute_allocate_slice(machine, instruction),
        Op::AllocateSharedSlice => super::execute_allocate_shared_slice(machine, instruction),
        Op::AllocateRaw => super::execute_allocate_raw(machine, instruction),
        Op::FreeRaw => super::execute_free_raw(machine, instruction),
        Op::FreeSharedRaw => super::execute_free_shared_raw(machine, instruction),
        Op::AllocateStack => super::execute_allocate_stack(machine, instruction),
        Op::PinHeap => super::execute_pin_heap(machine, instruction),
        Op::PinSharedHeap => super::execute_pin_shared_heap(machine, instruction),
        Op::UnpinHeap => super::execute_unpin_heap(machine, instruction),
        Op::UnpinSharedHeap => super::execute_unpin_shared_heap(machine, instruction),
        Op::DropHeap => super::execute_drop_heap(machine, instruction),
        Op::DropSharedHeap => super::execute_drop_shared_heap(machine, instruction),
        Op::DropStack => super::execute_drop_stack(machine, instruction),
        Op::DropSlice => super::execute_drop_slice(machine, instruction),
        Op::DropSharedSlice => super::execute_drop_shared_slice(machine, instruction),
        Op::VectorAndBool => super::execute_vector_and_bool(machine, instruction),
        Op::TensorAndBool => super::execute_tensor_and_bool(machine, instruction),
        Op::VectorOrBool => super::execute_vector_or_bool(machine, instruction),
        Op::TensorOrBool => super::execute_tensor_or_bool(machine, instruction),
        Op::VectorXorBool => super::execute_vector_xor_bool(machine, instruction),
        Op::TensorXorBool => super::execute_tensor_xor_bool(machine, instruction),
        Op::VectorAddInt => super::execute_vector_add_int(machine, instruction),
        Op::TensorAddInt => super::execute_tensor_add_int(machine, instruction),
        Op::VectorSubInt => super::execute_vector_sub_int(machine, instruction),
        Op::TensorSubInt => super::execute_tensor_sub_int(machine, instruction),
        Op::VectorMulInt => super::execute_vector_mul_int(machine, instruction),
        Op::TensorMulInt => super::execute_tensor_mul_int(machine, instruction),
        Op::VectorDivInt => super::execute_vector_div_int(machine, instruction),
        Op::TensorDivInt => super::execute_tensor_div_int(machine, instruction),
        Op::VectorDivUint => super::execute_vector_div_uint(machine, instruction),
        Op::TensorDivUint => super::execute_tensor_div_uint(machine, instruction),
        Op::VectorRemInt => super::execute_vector_rem_int(machine, instruction),
        Op::TensorRemInt => super::execute_tensor_rem_int(machine, instruction),
        Op::VectorRemUint => super::execute_vector_rem_uint(machine, instruction),
        Op::TensorRemUint => super::execute_tensor_rem_uint(machine, instruction),
        Op::VectorAndInt => super::execute_vector_and_int(machine, instruction),
        Op::TensorAndInt => super::execute_tensor_and_int(machine, instruction),
        Op::VectorOrInt => super::execute_vector_or_int(machine, instruction),
        Op::TensorOrInt => super::execute_tensor_or_int(machine, instruction),
        Op::VectorXorInt => super::execute_vector_xor_int(machine, instruction),
        Op::TensorXorInt => super::execute_tensor_xor_int(machine, instruction),
        Op::VectorShlInt => super::execute_vector_shl_int(machine, instruction),
        Op::TensorShlInt => super::execute_tensor_shl_int(machine, instruction),
        Op::VectorShrInt => super::execute_vector_shr_int(machine, instruction),
        Op::TensorShrInt => super::execute_tensor_shr_int(machine, instruction),
        Op::VectorShrUint => super::execute_vector_shr_uint(machine, instruction),
        Op::TensorShrUint => super::execute_tensor_shr_uint(machine, instruction),
        Op::VectorAddF32 => super::execute_vector_add_f32(machine, instruction),
        Op::TensorAddF32 => super::execute_tensor_add_f32(machine, instruction),
        Op::VectorAddF64 => super::execute_vector_add_f64(machine, instruction),
        Op::TensorAddF64 => super::execute_tensor_add_f64(machine, instruction),
        Op::VectorSubF32 => super::execute_vector_sub_f32(machine, instruction),
        Op::TensorSubF32 => super::execute_tensor_sub_f32(machine, instruction),
        Op::VectorSubF64 => super::execute_vector_sub_f64(machine, instruction),
        Op::TensorSubF64 => super::execute_tensor_sub_f64(machine, instruction),
        Op::VectorMulF32 => super::execute_vector_mul_f32(machine, instruction),
        Op::TensorMulF32 => super::execute_tensor_mul_f32(machine, instruction),
        Op::VectorMulF64 => super::execute_vector_mul_f64(machine, instruction),
        Op::TensorMulF64 => super::execute_tensor_mul_f64(machine, instruction),
        Op::VectorDivF32 => super::execute_vector_div_f32(machine, instruction),
        Op::TensorDivF32 => super::execute_tensor_div_f32(machine, instruction),
        Op::VectorDivF64 => super::execute_vector_div_f64(machine, instruction),
        Op::TensorDivF64 => super::execute_tensor_div_f64(machine, instruction),
        Op::VectorEqInt => super::execute_vector_eq_int(machine, instruction),
        Op::TensorEqInt => super::execute_tensor_eq_int(machine, instruction),
        Op::VectorEqBool => super::execute_vector_eq_bool(machine, instruction),
        Op::TensorEqBool => super::execute_tensor_eq_bool(machine, instruction),
        Op::VectorNeInt => super::execute_vector_ne_int(machine, instruction),
        Op::TensorNeInt => super::execute_tensor_ne_int(machine, instruction),
        Op::VectorNeBool => super::execute_vector_ne_bool(machine, instruction),
        Op::TensorNeBool => super::execute_tensor_ne_bool(machine, instruction),
        Op::VectorLtInt => super::execute_vector_lt_int(machine, instruction),
        Op::TensorLtInt => super::execute_tensor_lt_int(machine, instruction),
        Op::VectorLtUint => super::execute_vector_lt_uint(machine, instruction),
        Op::TensorLtUint => super::execute_tensor_lt_uint(machine, instruction),
        Op::VectorLeInt => super::execute_vector_le_int(machine, instruction),
        Op::TensorLeInt => super::execute_tensor_le_int(machine, instruction),
        Op::VectorLeUint => super::execute_vector_le_uint(machine, instruction),
        Op::TensorLeUint => super::execute_tensor_le_uint(machine, instruction),
        Op::VectorGtInt => super::execute_vector_gt_int(machine, instruction),
        Op::TensorGtInt => super::execute_tensor_gt_int(machine, instruction),
        Op::VectorGtUint => super::execute_vector_gt_uint(machine, instruction),
        Op::TensorGtUint => super::execute_tensor_gt_uint(machine, instruction),
        Op::VectorGeInt => super::execute_vector_ge_int(machine, instruction),
        Op::TensorGeInt => super::execute_tensor_ge_int(machine, instruction),
        Op::VectorGeUint => super::execute_vector_ge_uint(machine, instruction),
        Op::TensorGeUint => super::execute_tensor_ge_uint(machine, instruction),
        Op::VectorEqF32 => super::execute_vector_eq_f32(machine, instruction),
        Op::TensorEqF32 => super::execute_tensor_eq_f32(machine, instruction),
        Op::VectorEqF64 => super::execute_vector_eq_f64(machine, instruction),
        Op::TensorEqF64 => super::execute_tensor_eq_f64(machine, instruction),
        Op::VectorNeF32 => super::execute_vector_ne_f32(machine, instruction),
        Op::TensorNeF32 => super::execute_tensor_ne_f32(machine, instruction),
        Op::VectorNeF64 => super::execute_vector_ne_f64(machine, instruction),
        Op::TensorNeF64 => super::execute_tensor_ne_f64(machine, instruction),
        Op::VectorLtF32 => super::execute_vector_lt_f32(machine, instruction),
        Op::TensorLtF32 => super::execute_tensor_lt_f32(machine, instruction),
        Op::VectorLtF64 => super::execute_vector_lt_f64(machine, instruction),
        Op::TensorLtF64 => super::execute_tensor_lt_f64(machine, instruction),
        Op::VectorLeF32 => super::execute_vector_le_f32(machine, instruction),
        Op::TensorLeF32 => super::execute_tensor_le_f32(machine, instruction),
        Op::VectorLeF64 => super::execute_vector_le_f64(machine, instruction),
        Op::TensorLeF64 => super::execute_tensor_le_f64(machine, instruction),
        Op::VectorGtF32 => super::execute_vector_gt_f32(machine, instruction),
        Op::TensorGtF32 => super::execute_tensor_gt_f32(machine, instruction),
        Op::VectorGtF64 => super::execute_vector_gt_f64(machine, instruction),
        Op::TensorGtF64 => super::execute_tensor_gt_f64(machine, instruction),
        Op::VectorGeF32 => super::execute_vector_ge_f32(machine, instruction),
        Op::TensorGeF32 => super::execute_tensor_ge_f32(machine, instruction),
        Op::VectorGeF64 => super::execute_vector_ge_f64(machine, instruction),
        Op::TensorGeF64 => super::execute_tensor_ge_f64(machine, instruction),
        Op::AndBool => super::execute_and_bool(machine, instruction),
        Op::OrBool => super::execute_or_bool(machine, instruction),
        Op::XorBool => super::execute_xor_bool(machine, instruction),
        Op::AddI32 => super::execute_add_i32(machine, instruction),
        Op::AddU32 => super::execute_add_u32(machine, instruction),
        Op::AddI64 => super::execute_add_i64(machine, instruction),
        Op::AddU64 => super::execute_add_u64(machine, instruction),
        Op::SubI32 => super::execute_sub_i32(machine, instruction),
        Op::SubU32 => super::execute_sub_u32(machine, instruction),
        Op::SubI64 => super::execute_sub_i64(machine, instruction),
        Op::SubU64 => super::execute_sub_u64(machine, instruction),
        Op::MulI32 => super::execute_mul_i32(machine, instruction),
        Op::MulU32 => super::execute_mul_u32(machine, instruction),
        Op::MulI64 => super::execute_mul_i64(machine, instruction),
        Op::MulU64 => super::execute_mul_u64(machine, instruction),
        Op::DivI32 => super::execute_div_i32(machine, instruction),
        Op::DivU32 => super::execute_div_u32(machine, instruction),
        Op::DivI64 => super::execute_div_i64(machine, instruction),
        Op::DivU64 => super::execute_div_u64(machine, instruction),
        Op::RemI32 => super::execute_rem_i32(machine, instruction),
        Op::RemU32 => super::execute_rem_u32(machine, instruction),
        Op::RemI64 => super::execute_rem_i64(machine, instruction),
        Op::RemU64 => super::execute_rem_u64(machine, instruction),
        Op::AddInt => super::execute_add_int(machine, instruction),
        Op::AddUint => super::execute_add_uint(machine, instruction),
        Op::SubInt => super::execute_sub_int(machine, instruction),
        Op::SubUint => super::execute_sub_uint(machine, instruction),
        Op::MulInt => super::execute_mul_int(machine, instruction),
        Op::MulUint => super::execute_mul_uint(machine, instruction),
        Op::DivInt => super::execute_div_int(machine, instruction),
        Op::DivUint => super::execute_div_uint(machine, instruction),
        Op::RemInt => super::execute_rem_int(machine, instruction),
        Op::RemUint => super::execute_rem_uint(machine, instruction),
        Op::And32 => super::execute_and_32(machine, instruction),
        Op::And64 => super::execute_and_64(machine, instruction),
        Op::Or32 => super::execute_or_32(machine, instruction),
        Op::Or64 => super::execute_or_64(machine, instruction),
        Op::Xor32 => super::execute_xor_32(machine, instruction),
        Op::Xor64 => super::execute_xor_64(machine, instruction),
        Op::Shl32 => super::execute_shl_32(machine, instruction),
        Op::Shl64 => super::execute_shl_64(machine, instruction),
        Op::ShrI32 => super::execute_shr_i32(machine, instruction),
        Op::ShrU32 => super::execute_shr_u32(machine, instruction),
        Op::ShrI64 => super::execute_shr_i64(machine, instruction),
        Op::ShrU64 => super::execute_shr_u64(machine, instruction),
        Op::AddWideInt => super::execute_add_wide_int(machine, instruction),
        Op::SubWideInt => super::execute_sub_wide_int(machine, instruction),
        Op::MulWideInt => super::execute_mul_wide_int(machine, instruction),
        Op::DivWideInt => super::execute_div_wide_int(machine, instruction),
        Op::DivWideUint => super::execute_div_wide_uint(machine, instruction),
        Op::RemWideInt => super::execute_rem_wide_int(machine, instruction),
        Op::RemWideUint => super::execute_rem_wide_uint(machine, instruction),
        Op::AndWord => super::execute_and_word(machine, instruction),
        Op::OrWord => super::execute_or_word(machine, instruction),
        Op::XorWord => super::execute_xor_word(machine, instruction),
        Op::ShlWord => super::execute_shl_word(machine, instruction),
        Op::ShrInt => super::execute_shr_int(machine, instruction),
        Op::ShrUint => super::execute_shr_uint(machine, instruction),
        Op::AndWideInt => super::execute_and_wide_int(machine, instruction),
        Op::OrWideInt => super::execute_or_wide_int(machine, instruction),
        Op::XorWideInt => super::execute_xor_wide_int(machine, instruction),
        Op::ShlWideInt => super::execute_shl_wide_int(machine, instruction),
        Op::ShrWideInt => super::execute_shr_wide_int(machine, instruction),
        Op::ShrWideUint => super::execute_shr_wide_uint(machine, instruction),
        Op::AddF32 => super::execute_add_f32(machine, instruction),
        Op::SubF32 => super::execute_sub_f32(machine, instruction),
        Op::MulF32 => super::execute_mul_f32(machine, instruction),
        Op::DivF32 => super::execute_div_f32(machine, instruction),
        Op::EqF32 => super::execute_eq_f32(machine, instruction),
        Op::NeF32 => super::execute_ne_f32(machine, instruction),
        Op::LtF32 => super::execute_lt_f32(machine, instruction),
        Op::LeF32 => super::execute_le_f32(machine, instruction),
        Op::GtF32 => super::execute_gt_f32(machine, instruction),
        Op::GeF32 => super::execute_ge_f32(machine, instruction),
        Op::AddF64 => super::execute_add_f64(machine, instruction),
        Op::SubF64 => super::execute_sub_f64(machine, instruction),
        Op::MulF64 => super::execute_mul_f64(machine, instruction),
        Op::DivF64 => super::execute_div_f64(machine, instruction),
        Op::EqF64 => super::execute_eq_f64(machine, instruction),
        Op::NeF64 => super::execute_ne_f64(machine, instruction),
        Op::LtF64 => super::execute_lt_f64(machine, instruction),
        Op::LeF64 => super::execute_le_f64(machine, instruction),
        Op::GtF64 => super::execute_gt_f64(machine, instruction),
        Op::GeF64 => super::execute_ge_f64(machine, instruction),
        Op::Eq32 => super::execute_eq_32(machine, instruction),
        Op::Eq64 => super::execute_eq_64(machine, instruction),
        Op::Ne32 => super::execute_ne_32(machine, instruction),
        Op::Ne64 => super::execute_ne_64(machine, instruction),
        Op::LtI32 => super::execute_lt_i32(machine, instruction),
        Op::LtU32 => super::execute_lt_u32(machine, instruction),
        Op::LtI64 => super::execute_lt_i64(machine, instruction),
        Op::LtU64 => super::execute_lt_u64(machine, instruction),
        Op::LeI32 => super::execute_le_i32(machine, instruction),
        Op::LeU32 => super::execute_le_u32(machine, instruction),
        Op::LeI64 => super::execute_le_i64(machine, instruction),
        Op::LeU64 => super::execute_le_u64(machine, instruction),
        Op::GtI32 => super::execute_gt_i32(machine, instruction),
        Op::GtU32 => super::execute_gt_u32(machine, instruction),
        Op::GtI64 => super::execute_gt_i64(machine, instruction),
        Op::GtU64 => super::execute_gt_u64(machine, instruction),
        Op::GeI32 => super::execute_ge_i32(machine, instruction),
        Op::GeU32 => super::execute_ge_u32(machine, instruction),
        Op::GeI64 => super::execute_ge_i64(machine, instruction),
        Op::GeU64 => super::execute_ge_u64(machine, instruction),
        Op::EqInt => super::execute_eq_int(machine, instruction),
        Op::NeInt => super::execute_ne_int(machine, instruction),
        Op::LtInt => super::execute_lt_int(machine, instruction),
        Op::LtUint => super::execute_lt_uint(machine, instruction),
        Op::LeInt => super::execute_le_int(machine, instruction),
        Op::LeUint => super::execute_le_uint(machine, instruction),
        Op::GtInt => super::execute_gt_int(machine, instruction),
        Op::GtUint => super::execute_gt_uint(machine, instruction),
        Op::GeInt => super::execute_ge_int(machine, instruction),
        Op::GeUint => super::execute_ge_uint(machine, instruction),
        Op::EqWideInt => super::execute_eq_wide_int(machine, instruction),
        Op::NeWideInt => super::execute_ne_wide_int(machine, instruction),
        Op::LtWideInt => super::execute_lt_wide_int(machine, instruction),
        Op::LtWideUint => super::execute_lt_wide_uint(machine, instruction),
        Op::LeWideInt => super::execute_le_wide_int(machine, instruction),
        Op::LeWideUint => super::execute_le_wide_uint(machine, instruction),
        Op::GtWideInt => super::execute_gt_wide_int(machine, instruction),
        Op::GtWideUint => super::execute_gt_wide_uint(machine, instruction),
        Op::GeWideInt => super::execute_ge_wide_int(machine, instruction),
        Op::GeWideUint => super::execute_ge_wide_uint(machine, instruction),
        Op::NegI32 => super::execute_neg_i32(machine, instruction),
        Op::NegI64 => super::execute_neg_i64(machine, instruction),
        Op::Not32 => super::execute_not_32(machine, instruction),
        Op::Not64 => super::execute_not_64(machine, instruction),
        Op::NegInt => super::execute_neg_int(machine, instruction),
        Op::NotWord => super::execute_not_word(machine, instruction),
        Op::NegWideInt => super::execute_neg_wide_int(machine, instruction),
        Op::NotWideInt => super::execute_not_wide_int(machine, instruction),
        Op::NegF32 => super::execute_neg_f32(machine, instruction),
        Op::NegF64 => super::execute_neg_f64(machine, instruction),
        Op::NotBool => super::execute_not_bool(machine, instruction),
        Op::VectorNegInt => super::execute_vector_neg_int(machine, instruction),
        Op::TensorNegInt => super::execute_tensor_neg_int(machine, instruction),
        Op::VectorNotInt => super::execute_vector_not_int(machine, instruction),
        Op::TensorNotInt => super::execute_tensor_not_int(machine, instruction),
        Op::VectorNegF32 => super::execute_vector_neg_f32(machine, instruction),
        Op::TensorNegF32 => super::execute_tensor_neg_f32(machine, instruction),
        Op::VectorNegF64 => super::execute_vector_neg_f64(machine, instruction),
        Op::TensorNegF64 => super::execute_tensor_neg_f64(machine, instruction),
        Op::VectorNotBool => super::execute_vector_not_bool(machine, instruction),
        Op::TensorNotBool => super::execute_tensor_not_bool(machine, instruction),
        Op::CastBitcast => super::execute_cast_bitcast(machine, instruction),
        Op::CastTruncate => super::execute_cast_truncate(machine, instruction),
        Op::CastZeroExtend => super::execute_cast_zero_extend(machine, instruction),
        Op::CastSignExtend => super::execute_cast_sign_extend(machine, instruction),
        Op::CastFloatToSignedInt => super::execute_cast_float_to_signed_int(machine, instruction),
        Op::CastFloatToUnsignedInt => {
            super::execute_cast_float_to_unsigned_int(machine, instruction)
        }
        Op::CastFloatToSignedIntSaturating => {
            super::execute_cast_float_to_signed_int_saturating(machine, instruction)
        }
        Op::CastFloatToUnsignedIntSaturating => {
            super::execute_cast_float_to_unsigned_int_saturating(machine, instruction)
        }
        Op::CastSignedIntToF32 => super::execute_cast_signed_int_to_f32(machine, instruction),
        Op::CastSignedIntToF64 => super::execute_cast_signed_int_to_f64(machine, instruction),
        Op::CastUnsignedIntToF32 => super::execute_cast_unsigned_int_to_f32(machine, instruction),
        Op::CastUnsignedIntToF64 => super::execute_cast_unsigned_int_to_f64(machine, instruction),
        Op::CastFloatTruncate => super::execute_cast_float_truncate(machine, instruction),
        Op::CastFloatExtend => super::execute_cast_float_extend(machine, instruction),
        Op::CastPointerToInt => super::execute_cast_pointer_to_int(machine, instruction),
        Op::CastIntToPointer => super::execute_cast_int_to_pointer(machine, instruction),
        Op::CastWordToWideInt => super::execute_cast_word_to_wide_int(machine, instruction),
        Op::CastWideIntToWord => super::execute_cast_wide_int_to_word(machine, instruction),
        Op::CastWideInt => super::execute_cast_wide_int(machine, instruction),
        Op::Call => super::execute_call(machine, instruction, block_pc),
        Op::Invoke => super::execute_invoke(machine, instruction),
        Op::CallIndirect => super::execute_call_indirect(machine, instruction, block_pc),
        Op::CallCallable => super::execute_call_callable(machine, instruction, block_pc),
        Op::InvokeIndirect => super::execute_invoke_indirect(machine, instruction),
        Op::InvokeCallable => super::execute_invoke_callable(machine, instruction),
        Op::CallVirtualHeap => super::execute_call_virtual_heap(machine, instruction, block_pc),
        Op::CallVirtualSharedHeap => {
            super::execute_call_virtual_shared_heap(machine, instruction, block_pc)
        }
        Op::InvokeVirtualHeap => super::execute_invoke_virtual_heap(machine, instruction),
        Op::InvokeVirtualSharedHeap => {
            super::execute_invoke_virtual_shared_heap(machine, instruction)
        }
        Op::CallInterfaceHeap => super::execute_call_interface_heap(machine, instruction, block_pc),
        Op::CallInterfaceSharedHeap => {
            super::execute_call_interface_shared_heap(machine, instruction, block_pc)
        }
        Op::InvokeInterfaceHeap => super::execute_invoke_interface_heap(machine, instruction),
        Op::InvokeInterfaceSharedHeap => {
            super::execute_invoke_interface_shared_heap(machine, instruction)
        }
        Op::TailCall => super::execute_tail_call(machine, instruction),
        Op::TailCallSelf => super::execute_tail_call_self(machine, instruction),
        Op::TailCallIndirect => super::execute_tail_call_indirect(machine, instruction),
        Op::TailCallCallable => super::execute_tail_call_callable(machine, instruction),
        Op::TailCallVirtualHeap => super::execute_tail_call_virtual_heap(machine, instruction),
        Op::TailCallVirtualSharedHeap => {
            super::execute_tail_call_virtual_shared_heap(machine, instruction)
        }
        Op::TailCallInterfaceHeap => super::execute_tail_call_interface_heap(machine, instruction),
        Op::TailCallInterfaceSharedHeap => {
            super::execute_tail_call_interface_shared_heap(machine, instruction)
        }
        Op::Jump => super::execute_jump(machine, instruction),
        Op::BranchBool => super::execute_branch_bool(machine, instruction),
        Op::BranchEq32 => super::execute_branch_eq_32(machine, instruction),
        Op::BranchEq64 => super::execute_branch_eq_64(machine, instruction),
        Op::BranchNe32 => super::execute_branch_ne_32(machine, instruction),
        Op::BranchNe64 => super::execute_branch_ne_64(machine, instruction),
        Op::BranchLtI32 => super::execute_branch_lt_i32(machine, instruction),
        Op::BranchLtU32 => super::execute_branch_lt_u32(machine, instruction),
        Op::BranchLtI64 => super::execute_branch_lt_i64(machine, instruction),
        Op::BranchLtU64 => super::execute_branch_lt_u64(machine, instruction),
        Op::BranchLeI32 => super::execute_branch_le_i32(machine, instruction),
        Op::BranchLeU32 => super::execute_branch_le_u32(machine, instruction),
        Op::BranchLeI64 => super::execute_branch_le_i64(machine, instruction),
        Op::BranchLeU64 => super::execute_branch_le_u64(machine, instruction),
        Op::BranchGtI32 => super::execute_branch_gt_i32(machine, instruction),
        Op::BranchGtU32 => super::execute_branch_gt_u32(machine, instruction),
        Op::BranchGtI64 => super::execute_branch_gt_i64(machine, instruction),
        Op::BranchGtU64 => super::execute_branch_gt_u64(machine, instruction),
        Op::BranchGeI32 => super::execute_branch_ge_i32(machine, instruction),
        Op::BranchGeU32 => super::execute_branch_ge_u32(machine, instruction),
        Op::BranchGeI64 => super::execute_branch_ge_i64(machine, instruction),
        Op::BranchGeU64 => super::execute_branch_ge_u64(machine, instruction),
        Op::BranchEqInt => super::execute_branch_eq_int(machine, instruction),
        Op::BranchNeInt => super::execute_branch_ne_int(machine, instruction),
        Op::BranchLtInt => super::execute_branch_lt_int(machine, instruction),
        Op::BranchLeInt => super::execute_branch_le_int(machine, instruction),
        Op::BranchGtInt => super::execute_branch_gt_int(machine, instruction),
        Op::BranchGeInt => super::execute_branch_ge_int(machine, instruction),
        Op::BranchLtUint => super::execute_branch_lt_uint(machine, instruction),
        Op::BranchLeUint => super::execute_branch_le_uint(machine, instruction),
        Op::BranchGtUint => super::execute_branch_gt_uint(machine, instruction),
        Op::BranchGeUint => super::execute_branch_ge_uint(machine, instruction),
        Op::BranchEqF32 => super::execute_branch_eq_f32(machine, instruction),
        Op::BranchNeF32 => super::execute_branch_ne_f32(machine, instruction),
        Op::BranchLtF32 => super::execute_branch_lt_f32(machine, instruction),
        Op::BranchLeF32 => super::execute_branch_le_f32(machine, instruction),
        Op::BranchGtF32 => super::execute_branch_gt_f32(machine, instruction),
        Op::BranchGeF32 => super::execute_branch_ge_f32(machine, instruction),
        Op::BranchEqF64 => super::execute_branch_eq_f64(machine, instruction),
        Op::BranchNeF64 => super::execute_branch_ne_f64(machine, instruction),
        Op::BranchLtF64 => super::execute_branch_lt_f64(machine, instruction),
        Op::BranchLeF64 => super::execute_branch_le_f64(machine, instruction),
        Op::BranchGtF64 => super::execute_branch_gt_f64(machine, instruction),
        Op::BranchGeF64 => super::execute_branch_ge_f64(machine, instruction),
        Op::SwitchI32 | Op::SwitchI64 => super::execute_switch_word::<true>(machine, instruction),
        Op::SwitchU32 | Op::SwitchU64 => super::execute_switch_word::<false>(machine, instruction),
        Op::SwitchWideInt => super::execute_switch_wide::<true>(machine, instruction),
        Op::SwitchWideUint => super::execute_switch_wide::<false>(machine, instruction),
        Op::SwitchTableI32 | Op::SwitchTableI64 => {
            super::execute_switch_table_word::<true>(machine, instruction)
        }
        Op::SwitchTableU32 | Op::SwitchTableU64 => {
            super::execute_switch_table_word::<false>(machine, instruction)
        }
        Op::SwitchTableWideInt => super::execute_switch_table_wide::<true>(machine, instruction),
        Op::SwitchTableWideUint => super::execute_switch_table_wide::<false>(machine, instruction),
        Op::Assume => super::execute_assume(machine, instruction),
        Op::AtomicCompareExchange => super::execute_atomic_compare_exchange(machine, instruction),
        Op::AtomicFence => super::execute_atomic_fence(machine, instruction),
        Op::AtomicLoad => super::execute_atomic_load(machine, instruction),
        Op::AtomicStore => super::execute_atomic_store(machine, instruction),
        Op::AtomicExchange => super::execute_atomic_exchange(machine, instruction),
        Op::AtomicAdd => super::execute_atomic_add(machine, instruction),
        Op::AtomicSub => super::execute_atomic_sub(machine, instruction),
        Op::AtomicAnd => super::execute_atomic_and(machine, instruction),
        Op::AtomicOr => super::execute_atomic_or(machine, instruction),
        Op::AtomicXor => super::execute_atomic_xor(machine, instruction),
        Op::AtomicMin => super::execute_atomic_min(machine, instruction),
        Op::AtomicMax => super::execute_atomic_max(machine, instruction),
        Op::AtomicUmin => super::execute_atomic_umin(machine, instruction),
        Op::AtomicUmax => super::execute_atomic_umax(machine, instruction),
        Op::AtomicFadd => super::execute_atomic_fadd(machine, instruction),
        Op::AtomicFmin => super::execute_atomic_fmin(machine, instruction),
        Op::AtomicFmax => super::execute_atomic_fmax(machine, instruction),
        Op::BarrierWriteHeap => super::execute_barrier_write_heap(machine, instruction),
        Op::BarrierWriteSharedHeap => {
            super::execute_barrier_write_shared_heap(machine, instruction)
        }
        Op::Check => super::execute_check(machine, instruction),
        Op::IntrinsicLeadingZeroCount => {
            super::execute_intrinsic_leading_zero_count(machine, instruction)
        }
        Op::IntrinsicTrailingZeroCount => {
            super::execute_intrinsic_trailing_zero_count(machine, instruction)
        }
        Op::IntrinsicPopulationCount => {
            super::execute_intrinsic_population_count(machine, instruction)
        }
        Op::IntrinsicByteSwap => super::execute_intrinsic_byte_swap(machine, instruction),
        Op::IntrinsicBitReverse => super::execute_intrinsic_bit_reverse(machine, instruction),
        Op::IntrinsicRotateLeft => super::execute_intrinsic_rotate_left(machine, instruction),
        Op::IntrinsicRotateRight => super::execute_intrinsic_rotate_right(machine, instruction),
        Op::IntrinsicAddOverflow => super::execute_intrinsic_add_overflow(machine, instruction),
        Op::IntrinsicSubOverflow => super::execute_intrinsic_sub_overflow(machine, instruction),
        Op::IntrinsicMulOverflow => super::execute_intrinsic_mul_overflow(machine, instruction),
        Op::IntrinsicAddUnchecked => super::execute_intrinsic_add_unchecked(machine, instruction),
        Op::IntrinsicSubUnchecked => super::execute_intrinsic_sub_unchecked(machine, instruction),
        Op::IntrinsicMulUnchecked => super::execute_intrinsic_mul_unchecked(machine, instruction),
        Op::IntrinsicDivUnchecked => super::execute_intrinsic_div_unchecked(machine, instruction),
        Op::IntrinsicRemUnchecked => super::execute_intrinsic_rem_unchecked(machine, instruction),
        Op::IntrinsicShlUnchecked => super::execute_intrinsic_shl_unchecked(machine, instruction),
        Op::IntrinsicShrUnchecked => super::execute_intrinsic_shr_unchecked(machine, instruction),
        Op::IntrinsicSatAdd => super::execute_intrinsic_sat_add(machine, instruction),
        Op::IntrinsicSatSub => super::execute_intrinsic_sat_sub(machine, instruction),
        Op::IntrinsicMemcpy => super::execute_intrinsic_memcpy(machine, instruction),
        Op::IntrinsicMemmove => super::execute_intrinsic_memmove(machine, instruction),
        Op::IntrinsicMemset => super::execute_intrinsic_memset(machine, instruction),
        Op::IntrinsicMemcmp => super::execute_intrinsic_memcmp(machine, instruction),
        Op::IntrinsicPrefetchRead => super::execute_intrinsic_prefetch_read(machine, instruction),
        Op::IntrinsicPrefetchWrite => super::execute_intrinsic_prefetch_write(machine, instruction),
        Op::IntrinsicTransmute => super::execute_intrinsic_transmute(machine, instruction),
        Op::IntrinsicAddressSpaceCast => {
            super::execute_intrinsic_address_space_cast(machine, instruction)
        }
        Op::IntrinsicPointerOffsetFrom => {
            super::execute_intrinsic_pointer_offset_from(machine, instruction)
        }
        Op::IntrinsicRawEq => super::execute_intrinsic_raw_eq(machine, instruction),
        Op::IntrinsicSqrt => super::execute_intrinsic_sqrt(machine, instruction),
        Op::IntrinsicAbs => super::execute_intrinsic_abs(machine, instruction),
        Op::IntrinsicFma => super::execute_intrinsic_fma(machine, instruction),
        Op::IntrinsicCopySign => super::execute_intrinsic_copy_sign(machine, instruction),
        Op::IntrinsicMin => super::execute_intrinsic_min(machine, instruction),
        Op::IntrinsicMax => super::execute_intrinsic_max(machine, instruction),
        Op::IntrinsicSin => super::execute_intrinsic_sin(machine, instruction),
        Op::IntrinsicCos => super::execute_intrinsic_cos(machine, instruction),
        Op::IntrinsicTan => super::execute_intrinsic_tan(machine, instruction),
        Op::IntrinsicAsin => super::execute_intrinsic_asin(machine, instruction),
        Op::IntrinsicAcos => super::execute_intrinsic_acos(machine, instruction),
        Op::IntrinsicAtan => super::execute_intrinsic_atan(machine, instruction),
        Op::IntrinsicAtan2 => super::execute_intrinsic_atan2(machine, instruction),
        Op::IntrinsicExp => super::execute_intrinsic_exp(machine, instruction),
        Op::IntrinsicExp2 => super::execute_intrinsic_exp2(machine, instruction),
        Op::IntrinsicLog => super::execute_intrinsic_log(machine, instruction),
        Op::IntrinsicLog2 => super::execute_intrinsic_log2(machine, instruction),
        Op::IntrinsicLog10 => super::execute_intrinsic_log10(machine, instruction),
        Op::IntrinsicPow => super::execute_intrinsic_pow(machine, instruction),
        Op::IntrinsicFloor => super::execute_intrinsic_floor(machine, instruction),
        Op::IntrinsicCeil => super::execute_intrinsic_ceil(machine, instruction),
        Op::IntrinsicTrunc => super::execute_intrinsic_trunc(machine, instruction),
        Op::IntrinsicRound => super::execute_intrinsic_round(machine, instruction),
        Op::IntrinsicBreakpoint => super::execute_intrinsic_breakpoint(machine, instruction),
        Op::IntrinsicReturnAddress => super::execute_intrinsic_return_address(machine, instruction),
        Op::IntrinsicFrameAddress => super::execute_intrinsic_frame_address(machine, instruction),
        Op::IntrinsicExpect => super::execute_intrinsic_expect(machine, instruction),
        Op::IntrinsicBlackBox => super::execute_intrinsic_black_box(machine, instruction),
        Op::ReturnWord => super::execute_return_word(machine, instruction),
        Op::ReturnAddress => super::execute_return_address(machine, instruction),
        Op::ReturnVoid => super::execute_return_void(machine, instruction),
        Op::TensorBroadcast => super::execute_tensor_broadcast(machine, instruction),
        Op::TensorCast => super::execute_tensor_cast(machine, instruction),
        Op::TensorConcat => super::execute_tensor_concat(machine, instruction),
        Op::TensorConvertExact => super::execute_tensor_convert_exact(machine, instruction),
        Op::TensorConvertRoundTiesEven => {
            super::execute_tensor_convert_round_ties_even(machine, instruction)
        }
        Op::TensorConvertRoundTowardZero => {
            super::execute_tensor_convert_round_toward_zero(machine, instruction)
        }
        Op::TensorConvertRoundFloor => {
            super::execute_tensor_convert_round_floor(machine, instruction)
        }
        Op::TensorConvertRoundCeil => {
            super::execute_tensor_convert_round_ceil(machine, instruction)
        }
        Op::TensorConvertSaturate => super::execute_tensor_convert_saturate(machine, instruction),
        Op::TensorConvolution => super::execute_tensor_convolution(machine, instruction),
        Op::TensorCopy => super::execute_tensor_copy(machine, instruction),
        Op::TensorDot => super::execute_tensor_dot(machine, instruction),
        Op::TensorFill => super::execute_tensor_fill(machine, instruction),
        Op::TensorGather => super::execute_tensor_gather(machine, instruction),
        Op::TensorExtract => super::execute_tensor_extract(machine, instruction),
        Op::TensorLoad => super::execute_tensor_load(machine, instruction),
        Op::TensorSplat => super::execute_tensor_splat(machine, instruction),
        Op::TensorPad => super::execute_tensor_pad(machine, instruction),
        Op::TensorReduceAdd => super::execute_tensor_reduce_add(machine, instruction),
        Op::TensorReduceMultiply => super::execute_tensor_reduce_multiply(machine, instruction),
        Op::TensorReduceMin => super::execute_tensor_reduce_min(machine, instruction),
        Op::TensorReduceMax => super::execute_tensor_reduce_max(machine, instruction),
        Op::TensorReduceAnd => super::execute_tensor_reduce_and(machine, instruction),
        Op::TensorReduceOr => super::execute_tensor_reduce_or(machine, instruction),
        Op::TensorReduceXor => super::execute_tensor_reduce_xor(machine, instruction),
        Op::TensorReshape => super::execute_tensor_reshape(machine, instruction),
        Op::TensorScatterReplace => super::execute_tensor_scatter_replace(machine, instruction),
        Op::TensorScatterAdd => super::execute_tensor_scatter_add(machine, instruction),
        Op::TensorScatterMultiply => super::execute_tensor_scatter_multiply(machine, instruction),
        Op::TensorScatterMin => super::execute_tensor_scatter_min(machine, instruction),
        Op::TensorScatterMax => super::execute_tensor_scatter_max(machine, instruction),
        Op::TensorScatterAnd => super::execute_tensor_scatter_and(machine, instruction),
        Op::TensorScatterOr => super::execute_tensor_scatter_or(machine, instruction),
        Op::TensorScatterXor => super::execute_tensor_scatter_xor(machine, instruction),
        Op::TensorSelect => super::execute_tensor_select(machine, instruction),
        Op::TensorSlice => super::execute_tensor_slice(machine, instruction),
        Op::TensorStore => super::execute_tensor_store(machine, instruction),
        Op::TensorTranspose => super::execute_tensor_transpose(machine, instruction),
        Op::TensorView => super::execute_tensor_view(machine, instruction),
        Op::Panic => super::execute_panic(machine, instruction),
        Op::ThrowWord => super::execute_throw_word(machine, instruction),
        Op::ThrowAddress => super::execute_throw_address(machine, instruction),
        Op::Unreachable => super::execute_unreachable(machine, instruction),
        Op::VectorConvertExact => super::execute_vector_convert_exact(machine, instruction),
        Op::VectorConvertRoundTiesEven => {
            super::execute_vector_convert_round_ties_even(machine, instruction)
        }
        Op::VectorConvertRoundTowardZero => {
            super::execute_vector_convert_round_toward_zero(machine, instruction)
        }
        Op::VectorConvertRoundFloor => {
            super::execute_vector_convert_round_floor(machine, instruction)
        }
        Op::VectorConvertRoundCeil => {
            super::execute_vector_convert_round_ceil(machine, instruction)
        }
        Op::VectorConvertSaturate => super::execute_vector_convert_saturate(machine, instruction),
        Op::VectorExtract => super::execute_vector_extract(machine, instruction),
        Op::VectorInsert => super::execute_vector_insert(machine, instruction),
        Op::VectorReduceAdd => super::execute_vector_reduce_add(machine, instruction),
        Op::VectorReduceMultiply => super::execute_vector_reduce_multiply(machine, instruction),
        Op::VectorReduceMin => super::execute_vector_reduce_min(machine, instruction),
        Op::VectorReduceMax => super::execute_vector_reduce_max(machine, instruction),
        Op::VectorReduceAnd => super::execute_vector_reduce_and(machine, instruction),
        Op::VectorReduceOr => super::execute_vector_reduce_or(machine, instruction),
        Op::VectorReduceXor => super::execute_vector_reduce_xor(machine, instruction),
        Op::VectorSelect => super::execute_vector_select(machine, instruction),
        Op::VectorShuffle => super::execute_vector_shuffle(machine, instruction),
        Op::VectorSplat => super::execute_vector_splat(machine, instruction),
        Op::YieldWord => super::execute_yield_word(machine, instruction),
        Op::YieldAddress => super::execute_yield_address(machine, instruction),
    }
}
