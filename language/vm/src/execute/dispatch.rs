use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{Instruction, Opcode, Transfer};

/// Result from dispatching one block with instruction counting.
pub(crate) struct BlockDispatch {
    /// The control transfer produced by the block.
    pub(crate) transfer: Transfer,
    /// The number of opcodes executed.
    pub(crate) executed: u64,
}

/// Dispatch one block until it produces a control transfer.
pub(crate) fn dispatch_block(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let mut pc = pc;
    loop {
        if pc >= block.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }

        match dispatch_opcode(state, block, pc) {
            Transfer::Continue => {
                pc += 1;
            }
            transfer => return transfer,
        }
    }
}

/// Dispatch one block and count executed opcodes.
pub(crate) fn dispatch_block_counted(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> BlockDispatch {
    let mut pc = pc;
    let mut executed = 0;

    loop {
        if pc >= block.len() {
            return BlockDispatch {
                transfer: Transfer::Error(Error::InvalidInstruction),
                executed,
            };
        }

        executed += 1;
        match dispatch_opcode(state, block, pc) {
            Transfer::Continue => {
                pc += 1;
            }
            transfer => return BlockDispatch { transfer, executed },
        }
    }
}

/// Dispatch one program instruction.
#[inline]
fn dispatch_opcode(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let instruction = &block[pc];

    match instruction.opcode {
        Opcode::LoadConst => super::execute_load_const(state, instruction),
        Opcode::MoveFrame => super::execute_move_frame(state, instruction),
        Opcode::LoadFrameBytes => super::execute_load_frame_bytes(state, instruction),
        Opcode::StoreFrameBytes => super::execute_store_frame_bytes(state, instruction),
        Opcode::SelectWord => super::execute_select_word(state, instruction),
        Opcode::SelectFrame => super::execute_select_frame(state, instruction),
        Opcode::LoadLocal => super::execute_load_local(state, instruction),
        Opcode::StoreLocal => super::execute_store_local(state, instruction),
        Opcode::AddressLocal => super::execute_address_local(state, instruction),
        Opcode::AddressStatic => super::execute_address_static(state, instruction),
        Opcode::LoadStaticId => super::execute_load_static_id(state, instruction),
        Opcode::StoreStaticId => super::execute_store_static_id(state, instruction),
        Opcode::AddressFunction => super::execute_address_function(state, instruction),
        Opcode::BindCallable => super::execute_bind_callable(state, instruction),
        Opcode::LoadCallableEnvironment => {
            super::execute_load_callable_environment(state, instruction)
        }
        Opcode::LoadHeap => super::execute_load_heap(state, instruction),
        Opcode::LoadSharedHeap => super::execute_load_shared_heap(state, instruction),
        Opcode::LoadRaw => super::execute_load_raw(state, instruction),
        Opcode::LoadSharedRaw => super::execute_load_shared_raw(state, instruction),
        Opcode::LoadStack => super::execute_load_stack(state, instruction),
        Opcode::LoadFrame => super::execute_frame_load(state, instruction),
        Opcode::LoadFrameElement => super::execute_load_frame_element(state, instruction),
        Opcode::LoadStatic => super::execute_load_static(state, instruction),
        Opcode::StoreHeap => super::execute_store_heap(state, instruction),
        Opcode::StoreSharedHeap => super::execute_store_shared_heap(state, instruction),
        Opcode::StoreRaw => super::execute_store_raw(state, instruction),
        Opcode::StoreSharedRaw => super::execute_store_shared_raw(state, instruction),
        Opcode::StoreStack => super::execute_store_stack(state, instruction),
        Opcode::StoreFrame => super::execute_frame_store(state, instruction),
        Opcode::StoreFrameElement => super::execute_store_frame_element(state, instruction),
        Opcode::StoreStatic => super::execute_store_static(state, instruction),
        Opcode::AddressFrame => super::execute_address_frame(state, instruction),
        Opcode::AddressFrameElement => super::execute_address_frame_element(state, instruction),
        Opcode::AddressHeapField => super::execute_address_heap_field(state, instruction),
        Opcode::AddressSharedHeapField => {
            super::execute_address_shared_heap_field(state, instruction)
        }
        Opcode::AddressRawField => super::execute_address_raw_field(state, instruction),
        Opcode::AddressSharedRawField => {
            super::execute_address_shared_raw_field(state, instruction)
        }
        Opcode::AddressStackField => super::execute_address_stack_field(state, instruction),
        Opcode::AddressStaticField => super::execute_address_static_field(state, instruction),
        Opcode::LoadHeapField => super::execute_load_heap_field(state, instruction),
        Opcode::LoadSharedHeapField => super::execute_load_shared_heap_field(state, instruction),
        Opcode::LoadRawField => super::execute_load_raw_field(state, instruction),
        Opcode::LoadSharedRawField => super::execute_load_shared_raw_field(state, instruction),
        Opcode::LoadStackField => super::execute_load_stack_field(state, instruction),
        Opcode::LoadStaticField => super::execute_load_static_field(state, instruction),
        Opcode::StoreHeapField => super::execute_store_heap_field(state, instruction),
        Opcode::StoreSharedHeapField => super::execute_store_shared_heap_field(state, instruction),
        Opcode::StoreRawField => super::execute_store_raw_field(state, instruction),
        Opcode::StoreSharedRawField => super::execute_store_shared_raw_field(state, instruction),
        Opcode::StoreStackField => super::execute_store_stack_field(state, instruction),
        Opcode::StoreStaticField => super::execute_store_static_field(state, instruction),
        Opcode::AddressHeapElement => super::execute_address_heap_element(state, instruction),
        Opcode::AddressSharedHeapElement => {
            super::execute_address_shared_heap_element(state, instruction)
        }
        Opcode::AddressRawElement => super::execute_address_raw_element(state, instruction),
        Opcode::AddressSharedRawElement => {
            super::execute_address_shared_raw_element(state, instruction)
        }
        Opcode::AddressStackElement => super::execute_address_stack_element(state, instruction),
        Opcode::AddressStaticElement => super::execute_address_static_element(state, instruction),
        Opcode::AddressSliceElement => super::execute_address_slice_element(state, instruction),
        Opcode::LoadHeapElement => super::execute_load_heap_element(state, instruction),
        Opcode::LoadSharedHeapElement => {
            super::execute_load_shared_heap_element(state, instruction)
        }
        Opcode::LoadRawElement => super::execute_load_raw_element(state, instruction),
        Opcode::LoadSharedRawElement => super::execute_load_shared_raw_element(state, instruction),
        Opcode::LoadStackElement => super::execute_load_stack_element(state, instruction),
        Opcode::LoadStaticElement => super::execute_load_static_element(state, instruction),
        Opcode::StoreHeapElement => super::execute_store_heap_element(state, instruction),
        Opcode::StoreSharedHeapElement => {
            super::execute_store_shared_heap_element(state, instruction)
        }
        Opcode::StoreRawElement => super::execute_store_raw_element(state, instruction),
        Opcode::StoreSharedRawElement => {
            super::execute_store_shared_raw_element(state, instruction)
        }
        Opcode::StoreStackElement => super::execute_store_stack_element(state, instruction),
        Opcode::StoreStaticElement => super::execute_store_static_element(state, instruction),
        Opcode::AllocateHeap => super::execute_allocate_heap(state, instruction),
        Opcode::AllocateSharedHeap => super::execute_allocate_shared_heap(state, instruction),
        Opcode::AllocateSlice => super::execute_allocate_slice(state, instruction),
        Opcode::AllocateRaw => super::execute_allocate_raw(state, instruction),
        Opcode::FreeRaw => super::execute_free_raw(state, instruction),
        Opcode::AllocateStack => super::execute_allocate_stack(state, instruction),
        Opcode::Dispose => super::execute_dispose(state, instruction),
        Opcode::AsyncDispose => super::execute_async_dispose(state, instruction),
        Opcode::Pin => super::execute_pin(state, instruction),
        Opcode::Unpin => super::execute_unpin(state, instruction),
        Opcode::Drop => super::execute_drop(state, instruction),
        Opcode::BinaryWideInt | Opcode::BinaryWideUint => super::execute_binary(state, instruction),
        Opcode::BinaryElementwise => super::execute_binary_elementwise(state, instruction),
        Opcode::AndBool => super::execute_and_bool(state, instruction),
        Opcode::OrBool => super::execute_or_bool(state, instruction),
        Opcode::XorBool => super::execute_xor_bool(state, instruction),
        Opcode::AddInt => super::execute_add_int(state, instruction),
        Opcode::SubInt => super::execute_sub_int(state, instruction),
        Opcode::MulInt => super::execute_mul_int(state, instruction),
        Opcode::DivInt => super::execute_div_int(state, instruction),
        Opcode::DivUint => super::execute_div_uint(state, instruction),
        Opcode::RemInt => super::execute_rem_int(state, instruction),
        Opcode::RemUint => super::execute_rem_uint(state, instruction),
        Opcode::AndInt => super::execute_and_int(state, instruction),
        Opcode::OrInt => super::execute_or_int(state, instruction),
        Opcode::XorInt => super::execute_xor_int(state, instruction),
        Opcode::ShlInt => super::execute_shl_int(state, instruction),
        Opcode::ShrInt => super::execute_shr_int(state, instruction),
        Opcode::ShrUint => super::execute_shr_uint(state, instruction),
        Opcode::AddF32 => super::execute_add_f32(state, instruction),
        Opcode::SubF32 => super::execute_sub_f32(state, instruction),
        Opcode::MulF32 => super::execute_mul_f32(state, instruction),
        Opcode::DivF32 => super::execute_div_f32(state, instruction),
        Opcode::EqF32 => super::execute_eq_f32(state, instruction),
        Opcode::NeF32 => super::execute_ne_f32(state, instruction),
        Opcode::LtF32 => super::execute_lt_f32(state, instruction),
        Opcode::LeF32 => super::execute_le_f32(state, instruction),
        Opcode::GtF32 => super::execute_gt_f32(state, instruction),
        Opcode::GeF32 => super::execute_ge_f32(state, instruction),
        Opcode::AddF64 => super::execute_add_f64(state, instruction),
        Opcode::SubF64 => super::execute_sub_f64(state, instruction),
        Opcode::MulF64 => super::execute_mul_f64(state, instruction),
        Opcode::DivF64 => super::execute_div_f64(state, instruction),
        Opcode::EqF64 => super::execute_eq_f64(state, instruction),
        Opcode::NeF64 => super::execute_ne_f64(state, instruction),
        Opcode::LtF64 => super::execute_lt_f64(state, instruction),
        Opcode::LeF64 => super::execute_le_f64(state, instruction),
        Opcode::GtF64 => super::execute_gt_f64(state, instruction),
        Opcode::GeF64 => super::execute_ge_f64(state, instruction),
        Opcode::EqInt => super::execute_eq_int(state, instruction),
        Opcode::NeInt => super::execute_ne_int(state, instruction),
        Opcode::LtInt => super::execute_lt_int(state, instruction),
        Opcode::LtUint => super::execute_lt_uint(state, instruction),
        Opcode::LeInt => super::execute_le_int(state, instruction),
        Opcode::LeUint => super::execute_le_uint(state, instruction),
        Opcode::GtInt => super::execute_gt_int(state, instruction),
        Opcode::GtUint => super::execute_gt_uint(state, instruction),
        Opcode::GeInt => super::execute_ge_int(state, instruction),
        Opcode::GeUint => super::execute_ge_uint(state, instruction),
        Opcode::UnaryWideInt => super::execute_unary(state, instruction),
        Opcode::NegInt => super::execute_neg_int(state, instruction),
        Opcode::NotInt => super::execute_not_int(state, instruction),
        Opcode::NegF32 => super::execute_neg_f32(state, instruction),
        Opcode::NegF64 => super::execute_neg_f64(state, instruction),
        Opcode::NotBool => super::execute_not_bool(state, instruction),
        Opcode::UnaryElementwise => super::execute_unary_elementwise(state, instruction),
        Opcode::CastWord => super::execute_cast_word(state, instruction),
        Opcode::CastWordToWideInt => super::execute_cast_word_to_wide_int(state, instruction),
        Opcode::CastWideIntToWord => super::execute_cast_wide_int_to_word(state, instruction),
        Opcode::CastWideInt => super::execute_cast_wide_int(state, instruction),
        Opcode::Call => super::execute_call(state, instruction, pc),
        Opcode::Invoke => super::execute_invoke(state, instruction),
        Opcode::CallIndirect => super::execute_call_indirect(state, instruction, pc),
        Opcode::InvokeIndirect => super::execute_invoke_indirect(state, instruction),
        Opcode::CallVirtual => super::execute_call_virtual(state, instruction, pc),
        Opcode::InvokeVirtual => super::execute_invoke_virtual(state, instruction),
        Opcode::CallInterface => super::execute_call_interface(state, instruction, pc),
        Opcode::InvokeInterface => super::execute_invoke_interface(state, instruction),
        Opcode::TailCall => super::execute_tail_call(state, instruction),
        Opcode::TailCallSelf => super::execute_tail_call_self(state, instruction),
        Opcode::TailCallIndirect => super::execute_tail_call_indirect(state, instruction),
        Opcode::TailCallVirtual => super::execute_tail_call_virtual(state, instruction),
        Opcode::TailCallInterface => super::execute_tail_call_interface(state, instruction),
        Opcode::Jump => super::execute_jump(state, instruction),
        Opcode::BranchBool => super::execute_branch_bool(state, instruction),
        Opcode::BranchEqInt => super::execute_branch_eq_int(state, instruction),
        Opcode::BranchNeInt => super::execute_branch_ne_int(state, instruction),
        Opcode::BranchLtInt => super::execute_branch_lt_int(state, instruction),
        Opcode::BranchLeInt => super::execute_branch_le_int(state, instruction),
        Opcode::BranchGtInt => super::execute_branch_gt_int(state, instruction),
        Opcode::BranchGeInt => super::execute_branch_ge_int(state, instruction),
        Opcode::BranchLtUint => super::execute_branch_lt_uint(state, instruction),
        Opcode::BranchLeUint => super::execute_branch_le_uint(state, instruction),
        Opcode::BranchGtUint => super::execute_branch_gt_uint(state, instruction),
        Opcode::BranchGeUint => super::execute_branch_ge_uint(state, instruction),
        Opcode::BranchEqF32 => super::execute_branch_eq_f32(state, instruction),
        Opcode::BranchNeF32 => super::execute_branch_ne_f32(state, instruction),
        Opcode::BranchLtF32 => super::execute_branch_lt_f32(state, instruction),
        Opcode::BranchLeF32 => super::execute_branch_le_f32(state, instruction),
        Opcode::BranchGtF32 => super::execute_branch_gt_f32(state, instruction),
        Opcode::BranchGeF32 => super::execute_branch_ge_f32(state, instruction),
        Opcode::BranchEqF64 => super::execute_branch_eq_f64(state, instruction),
        Opcode::BranchNeF64 => super::execute_branch_ne_f64(state, instruction),
        Opcode::BranchLtF64 => super::execute_branch_lt_f64(state, instruction),
        Opcode::BranchLeF64 => super::execute_branch_le_f64(state, instruction),
        Opcode::BranchGtF64 => super::execute_branch_gt_f64(state, instruction),
        Opcode::BranchGeF64 => super::execute_branch_ge_f64(state, instruction),
        Opcode::Switch32 | Opcode::Switch64 | Opcode::SwitchWideInt => {
            super::execute_switch(state, instruction)
        }
        Opcode::SwitchTable32 | Opcode::SwitchTable64 | Opcode::SwitchTableWideInt => {
            super::execute_switch_table(state, instruction)
        }
        Opcode::Assume => super::execute_assume(state, instruction),
        Opcode::AtomicCompareExchange => super::execute_atomic_compare_exchange(state, instruction),
        Opcode::AtomicFence => super::execute_atomic_fence(state, instruction),
        Opcode::AtomicLoad => super::execute_atomic_load(state, instruction),
        Opcode::AtomicRmw => super::execute_atomic_rmw(state, instruction),
        Opcode::AtomicStore => super::execute_atomic_store(state, instruction),
        Opcode::BarrierWrite => super::execute_barrier_write(state, instruction),
        Opcode::Check => super::execute_check(state, instruction),
        Opcode::Intrinsic => super::execute_intrinsic(state, instruction),
        Opcode::Return => super::execute_return(state, instruction),
        Opcode::TensorBroadcast => super::execute_tensor_broadcast(state, instruction),
        Opcode::TensorCast => super::execute_tensor_cast(state, instruction),
        Opcode::TensorCompare => super::execute_tensor_compare(state, instruction),
        Opcode::TensorConcat => super::execute_tensor_concat(state, instruction),
        Opcode::TensorConvert => super::execute_tensor_convert(state, instruction),
        Opcode::TensorConvolution => super::execute_tensor_convolution(state, instruction),
        Opcode::TensorCopy => super::execute_tensor_copy(state, instruction),
        Opcode::TensorDot => super::execute_tensor_dot(state, instruction),
        Opcode::TensorFill => super::execute_tensor_fill(state, instruction),
        Opcode::TensorGather => super::execute_tensor_gather(state, instruction),
        Opcode::TensorExtract => super::execute_tensor_extract(state, instruction),
        Opcode::TensorLoad => super::execute_tensor_load(state, instruction),
        Opcode::TensorSplat => super::execute_tensor_splat(state, instruction),
        Opcode::TensorPad => super::execute_tensor_pad(state, instruction),
        Opcode::TensorReduce => super::execute_tensor_reduce(state, instruction),
        Opcode::TensorReshape => super::execute_tensor_reshape(state, instruction),
        Opcode::TensorScatter => super::execute_tensor_scatter(state, instruction),
        Opcode::TensorSelect => super::execute_tensor_select(state, instruction),
        Opcode::TensorSlice => super::execute_tensor_slice(state, instruction),
        Opcode::TensorStore => super::execute_tensor_store(state, instruction),
        Opcode::TensorTranspose => super::execute_tensor_transpose(state, instruction),
        Opcode::TensorView => super::execute_tensor_view(state, instruction),
        Opcode::Trap => super::execute_trap(state, instruction),
        Opcode::Throw => super::execute_throw(state, instruction),
        Opcode::Unreachable => super::execute_unreachable(state, instruction),
        Opcode::VectorCompare => super::execute_vector_compare(state, instruction),
        Opcode::VectorConvert => super::execute_vector_convert(state, instruction),
        Opcode::VectorExtract => super::execute_vector_extract(state, instruction),
        Opcode::VectorInsert => super::execute_vector_insert(state, instruction),
        Opcode::VectorReduce => super::execute_vector_reduce(state, instruction),
        Opcode::VectorSelect => super::execute_vector_select(state, instruction),
        Opcode::VectorShuffle => super::execute_vector_shuffle(state, instruction),
        Opcode::VectorSplat => super::execute_vector_splat(state, instruction),
        Opcode::Yield => super::execute_yield(state, instruction),
    }
}
