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
        Op::LoadConst => super::execute_load_const(machine, instruction),
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
        Op::LoadStaticId => super::execute_load_static_id(machine, instruction),
        Op::StoreStaticId => super::execute_store_static_id(machine, instruction),
        Op::AddressFunction => super::execute_address_function(machine, instruction),
        Op::BindCallable => super::execute_bind_callable(machine, instruction),
        Op::LoadCallableEnvironment => {
            super::execute_load_callable_environment(machine, instruction)
        }
        Op::LoadHeap => super::execute_load_heap(machine, instruction),
        Op::LoadSharedHeap => super::execute_load_shared_heap(machine, instruction),
        Op::LoadRaw => super::execute_load_raw(machine, instruction),
        Op::LoadSharedRaw => super::execute_load_shared_raw(machine, instruction),
        Op::LoadStack => super::execute_load_stack(machine, instruction),
        Op::LoadFrame => super::execute_load_frame(machine, instruction),
        Op::LoadFrameElement => super::execute_load_frame_element(machine, instruction),
        Op::LoadStatic => super::execute_load_static(machine, instruction),
        Op::StoreHeap => super::execute_store_heap(machine, instruction),
        Op::StoreSharedHeap => super::execute_store_shared_heap(machine, instruction),
        Op::StoreRaw => super::execute_store_raw(machine, instruction),
        Op::StoreSharedRaw => super::execute_store_shared_raw(machine, instruction),
        Op::StoreStack => super::execute_store_stack(machine, instruction),
        Op::StoreFrame => super::execute_store_frame(machine, instruction),
        Op::StoreFrameElement => super::execute_store_frame_element(machine, instruction),
        Op::StoreStatic => super::execute_store_static(machine, instruction),
        Op::Abort => super::execute_abort(machine, instruction),
        Op::AddressFrame => super::execute_address_frame(machine, instruction),
        Op::AddressFrameElement => super::execute_address_frame_element(machine, instruction),
        Op::AddressHeapField => super::execute_address_heap_field(machine, instruction),
        Op::AddressSharedHeapField => {
            super::execute_address_shared_heap_field(machine, instruction)
        }
        Op::AddressRawField => super::execute_address_raw_field(machine, instruction),
        Op::AddressSharedRawField => super::execute_address_shared_raw_field(machine, instruction),
        Op::AddressStackField => super::execute_address_stack_field(machine, instruction),
        Op::AddressStaticField => super::execute_address_static_field(machine, instruction),
        Op::LoadHeapField => super::execute_load_heap_field(machine, instruction),
        Op::LoadSharedHeapField => super::execute_load_shared_heap_field(machine, instruction),
        Op::LoadRawField => super::execute_load_raw_field(machine, instruction),
        Op::LoadSharedRawField => super::execute_load_shared_raw_field(machine, instruction),
        Op::LoadStackField => super::execute_load_stack_field(machine, instruction),
        Op::LoadStaticField => super::execute_load_static_field(machine, instruction),
        Op::StoreHeapField => super::execute_store_heap_field(machine, instruction),
        Op::StoreSharedHeapField => super::execute_store_shared_heap_field(machine, instruction),
        Op::StoreRawField => super::execute_store_raw_field(machine, instruction),
        Op::StoreSharedRawField => super::execute_store_shared_raw_field(machine, instruction),
        Op::StoreStackField => super::execute_store_stack_field(machine, instruction),
        Op::StoreStaticField => super::execute_store_static_field(machine, instruction),
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
        Op::LoadHeapElement => super::execute_load_heap_element(machine, instruction),
        Op::LoadSharedHeapElement => super::execute_load_shared_heap_element(machine, instruction),
        Op::LoadRawElement => super::execute_load_raw_element(machine, instruction),
        Op::LoadSharedRawElement => super::execute_load_shared_raw_element(machine, instruction),
        Op::LoadStackElement => super::execute_load_stack_element(machine, instruction),
        Op::LoadStaticElement => super::execute_load_static_element(machine, instruction),
        Op::StoreHeapElement => super::execute_store_heap_element(machine, instruction),
        Op::StoreSharedHeapElement => {
            super::execute_store_shared_heap_element(machine, instruction)
        }
        Op::StoreRawElement => super::execute_store_raw_element(machine, instruction),
        Op::StoreSharedRawElement => super::execute_store_shared_raw_element(machine, instruction),
        Op::StoreStackElement => super::execute_store_stack_element(machine, instruction),
        Op::StoreStaticElement => super::execute_store_static_element(machine, instruction),
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
        Op::BinaryElementwise => super::execute_binary_elementwise(machine, instruction),
        Op::AndBool => super::execute_and_bool(machine, instruction),
        Op::OrBool => super::execute_or_bool(machine, instruction),
        Op::XorBool => super::execute_xor_bool(machine, instruction),
        Op::AddInt => super::execute_add_int(machine, instruction),
        Op::SubInt => super::execute_sub_int(machine, instruction),
        Op::MulInt => super::execute_mul_int(machine, instruction),
        Op::DivInt => super::execute_div_int(machine, instruction),
        Op::DivUint => super::execute_div_uint(machine, instruction),
        Op::RemInt => super::execute_rem_int(machine, instruction),
        Op::RemUint => super::execute_rem_uint(machine, instruction),
        Op::AddWideInt => super::execute_add_wide_int(machine, instruction),
        Op::SubWideInt => super::execute_sub_wide_int(machine, instruction),
        Op::MulWideInt => super::execute_mul_wide_int(machine, instruction),
        Op::DivWideInt => super::execute_div_wide_int(machine, instruction),
        Op::DivWideUint => super::execute_div_wide_uint(machine, instruction),
        Op::RemWideInt => super::execute_rem_wide_int(machine, instruction),
        Op::RemWideUint => super::execute_rem_wide_uint(machine, instruction),
        Op::AndInt => super::execute_and_int(machine, instruction),
        Op::OrInt => super::execute_or_int(machine, instruction),
        Op::XorInt => super::execute_xor_int(machine, instruction),
        Op::ShlInt => super::execute_shl_int(machine, instruction),
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
        Op::NegInt => super::execute_neg_int(machine, instruction),
        Op::NotInt => super::execute_not_int(machine, instruction),
        Op::NegWideInt => super::execute_neg_wide_int(machine, instruction),
        Op::NotWideInt => super::execute_not_wide_int(machine, instruction),
        Op::NegF32 => super::execute_neg_f32(machine, instruction),
        Op::NegF64 => super::execute_neg_f64(machine, instruction),
        Op::NotBool => super::execute_not_bool(machine, instruction),
        Op::UnaryElementwise => super::execute_unary_elementwise(machine, instruction),
        Op::CastWord => super::execute_cast_word(machine, instruction),
        Op::CastWordToWideInt => super::execute_cast_word_to_wide_int(machine, instruction),
        Op::CastWideIntToWord => super::execute_cast_wide_int_to_word(machine, instruction),
        Op::CastWideInt => super::execute_cast_wide_int(machine, instruction),
        Op::Call => super::execute_call(machine, instruction, block_pc),
        Op::Invoke => super::execute_invoke(machine, instruction),
        Op::CallIndirect => super::execute_call_indirect(machine, instruction, block_pc),
        Op::InvokeIndirect => super::execute_invoke_indirect(machine, instruction),
        Op::CallVirtual => super::execute_call_virtual(machine, instruction, block_pc),
        Op::InvokeVirtual => super::execute_invoke_virtual(machine, instruction),
        Op::CallInterface => super::execute_call_interface(machine, instruction, block_pc),
        Op::InvokeInterface => super::execute_invoke_interface(machine, instruction),
        Op::TailCall => super::execute_tail_call(machine, instruction),
        Op::TailCallSelf => super::execute_tail_call_self(machine, instruction),
        Op::TailCallIndirect => super::execute_tail_call_indirect(machine, instruction),
        Op::TailCallVirtual => super::execute_tail_call_virtual(machine, instruction),
        Op::TailCallInterface => super::execute_tail_call_interface(machine, instruction),
        Op::Jump => super::execute_jump(machine, instruction),
        Op::BranchBool => super::execute_branch_bool(machine, instruction),
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
        Op::Switch32 | Op::Switch64 | Op::SwitchWideInt => {
            super::execute_switch(machine, instruction)
        }
        Op::SwitchTable32 | Op::SwitchTable64 | Op::SwitchTableWideInt => {
            super::execute_switch_table(machine, instruction)
        }
        Op::Assume => super::execute_assume(machine, instruction),
        Op::AtomicCompareExchange => super::execute_atomic_compare_exchange(machine, instruction),
        Op::AtomicFence => super::execute_atomic_fence(machine, instruction),
        Op::AtomicLoad => super::execute_atomic_load(machine, instruction),
        Op::AtomicRmw => super::execute_atomic_rmw(machine, instruction),
        Op::AtomicStore => super::execute_atomic_store(machine, instruction),
        Op::BarrierWriteHeap => super::execute_barrier_write_heap(machine, instruction),
        Op::BarrierWriteSharedHeap => {
            super::execute_barrier_write_shared_heap(machine, instruction)
        }
        Op::Check => super::execute_check(machine, instruction),
        Op::Intrinsic => super::execute_intrinsic(machine, instruction),
        Op::Return => super::execute_return(machine, instruction),
        Op::ReturnVoid => super::execute_return_void(machine, instruction),
        Op::TensorBroadcast => super::execute_tensor_broadcast(machine, instruction),
        Op::TensorCast => super::execute_tensor_cast(machine, instruction),
        Op::TensorCompare => super::execute_tensor_compare(machine, instruction),
        Op::TensorConcat => super::execute_tensor_concat(machine, instruction),
        Op::TensorConvert => super::execute_tensor_convert(machine, instruction),
        Op::TensorConvolution => super::execute_tensor_convolution(machine, instruction),
        Op::TensorCopy => super::execute_tensor_copy(machine, instruction),
        Op::TensorDot => super::execute_tensor_dot(machine, instruction),
        Op::TensorFill => super::execute_tensor_fill(machine, instruction),
        Op::TensorGather => super::execute_tensor_gather(machine, instruction),
        Op::TensorExtract => super::execute_tensor_extract(machine, instruction),
        Op::TensorLoad => super::execute_tensor_load(machine, instruction),
        Op::TensorSplat => super::execute_tensor_splat(machine, instruction),
        Op::TensorPad => super::execute_tensor_pad(machine, instruction),
        Op::TensorReduce => super::execute_tensor_reduce(machine, instruction),
        Op::TensorReshape => super::execute_tensor_reshape(machine, instruction),
        Op::TensorScatter => super::execute_tensor_scatter(machine, instruction),
        Op::TensorSelect => super::execute_tensor_select(machine, instruction),
        Op::TensorSlice => super::execute_tensor_slice(machine, instruction),
        Op::TensorStore => super::execute_tensor_store(machine, instruction),
        Op::TensorTranspose => super::execute_tensor_transpose(machine, instruction),
        Op::TensorView => super::execute_tensor_view(machine, instruction),
        Op::Panic => super::execute_panic(machine, instruction),
        Op::Throw => super::execute_throw(machine, instruction),
        Op::Unreachable => super::execute_unreachable(machine, instruction),
        Op::VectorCompare => super::execute_vector_compare(machine, instruction),
        Op::VectorConvertExact
        | Op::VectorConvertRoundTiesEven
        | Op::VectorConvertRoundTowardZero
        | Op::VectorConvertRoundFloor
        | Op::VectorConvertRoundCeil
        | Op::VectorConvertSaturate => super::execute_vector_convert(machine, instruction),
        Op::VectorExtract => super::execute_vector_extract(machine, instruction),
        Op::VectorInsert => super::execute_vector_insert(machine, instruction),
        Op::VectorReduce => super::execute_vector_reduce(machine, instruction),
        Op::VectorSelect => super::execute_vector_select(machine, instruction),
        Op::VectorShuffle => super::execute_vector_shuffle(machine, instruction),
        Op::VectorSplat => super::execute_vector_splat(machine, instruction),
        Op::Yield => super::execute_yield(machine, instruction),
    }
}
