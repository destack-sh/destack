use crate::executable::{ControlFlow, Instruction, InstructionOperation};
use crate::interpreter::ExecutionState;

/// Dispatch one lowered instruction through the interpreter execute surface.
pub(crate) fn dispatch_instruction(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> ControlFlow {
    match block[pc].operation {
        InstructionOperation::AddConstInt => super::handle_add_const_int(state, block, pc),
        InstructionOperation::AddConstUint => super::handle_add_const_uint(state, block, pc),
        InstructionOperation::AddInt => super::handle_add_int(state, block, pc),
        InstructionOperation::AddUint => super::handle_add_uint(state, block, pc),
        InstructionOperation::Aggregate => super::handle_aggregate(state, block, pc),
        InstructionOperation::AndInt => super::handle_and_int(state, block, pc),
        InstructionOperation::AndUint => super::handle_and_uint(state, block, pc),
        InstructionOperation::Assume => super::handle_assume(state, block, pc),
        InstructionOperation::AtomicCompareExchange => {
            super::handle_atomic_compare_exchange(state, block, pc)
        }
        InstructionOperation::AtomicFence => super::handle_atomic_fence(state, block, pc),
        InstructionOperation::AtomicLoad => super::handle_atomic_load(state, block, pc),
        InstructionOperation::AtomicRmw => super::handle_atomic_rmw(state, block, pc),
        InstructionOperation::AtomicStore => super::handle_atomic_store(state, block, pc),
        InstructionOperation::Barrier => super::handle_barrier(state, block, pc),
        InstructionOperation::Binary => super::handle_binary(state, block, pc),
        InstructionOperation::BinaryBool => super::handle_binary_bool(state, block, pc),
        InstructionOperation::BinaryConstRight => {
            super::handle_binary_const_right(state, block, pc)
        }
        InstructionOperation::BinaryElementwise => {
            super::handle_binary_elementwise(state, block, pc)
        }
        InstructionOperation::BinaryFloat32 => super::handle_binary_float32(state, block, pc),
        InstructionOperation::BinaryFloat64 => super::handle_binary_float64(state, block, pc),
        InstructionOperation::BinaryInt => super::handle_binary_int(state, block, pc),
        InstructionOperation::BinaryUint => super::handle_binary_uint(state, block, pc),
        InstructionOperation::Branch => super::handle_branch(state, block, pc),
        InstructionOperation::BranchBool => super::handle_branch_bool(state, block, pc),
        InstructionOperation::Call => super::handle_call(state, block, pc),
        InstructionOperation::CallIndirect => super::handle_call_indirect(state, block, pc),
        InstructionOperation::CallInterface => super::handle_call_interface(state, block, pc),
        InstructionOperation::CallVirtual => super::handle_call_virtual(state, block, pc),
        InstructionOperation::Cast => super::handle_cast(state, block, pc),
        InstructionOperation::CompareAndBranch => {
            super::handle_compare_and_branch(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConst => {
            super::handle_compare_and_branch_const(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstFloat => {
            super::handle_compare_and_branch_const_float(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstInt => {
            super::handle_compare_and_branch_const_int(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstUint => {
            super::handle_compare_and_branch_const_uint(state, block, pc)
        }
        InstructionOperation::CompareAndBranchFloat => {
            super::handle_compare_and_branch_float(state, block, pc)
        }
        InstructionOperation::CompareAndBranchInt => {
            super::handle_compare_and_branch_int(state, block, pc)
        }
        InstructionOperation::CompareAndBranchUint => {
            super::handle_compare_and_branch_uint(state, block, pc)
        }
        InstructionOperation::Const => super::handle_const(state, block, pc),
        InstructionOperation::ElementAddr => super::handle_element_addr(state, block, pc),
        InstructionOperation::ElementAddrAggregate => {
            super::handle_element_addr_aggregate(state, block, pc)
        }
        InstructionOperation::ElementAddrGlobal => {
            super::handle_element_addr_global(state, block, pc)
        }
        InstructionOperation::ElementAddrManaged => {
            super::handle_element_addr_managed(state, block, pc)
        }
        InstructionOperation::ElementAddrRaw => super::handle_element_addr_raw(state, block, pc),
        InstructionOperation::ElementAddrStack => {
            super::handle_element_addr_stack(state, block, pc)
        }
        InstructionOperation::ElementGet => super::handle_element_get(state, block, pc),
        InstructionOperation::ElementLoad => super::handle_element_load(state, block, pc),
        InstructionOperation::ElementLoadAggregate => {
            super::handle_element_load_aggregate(state, block, pc)
        }
        InstructionOperation::ElementLoadGlobal => {
            super::handle_element_load_global(state, block, pc)
        }
        InstructionOperation::ElementLoadManaged => {
            super::handle_element_load_managed(state, block, pc)
        }
        InstructionOperation::ElementLoadRaw => super::handle_element_load_raw(state, block, pc),
        InstructionOperation::ElementLoadStack => {
            super::handle_element_load_stack(state, block, pc)
        }
        InstructionOperation::ElementSet => super::handle_element_set(state, block, pc),
        InstructionOperation::ElementStore => super::handle_element_store(state, block, pc),
        InstructionOperation::ElementStoreAggregate => {
            super::handle_element_store_aggregate(state, block, pc)
        }
        InstructionOperation::ElementStoreGlobal => {
            super::handle_element_store_global(state, block, pc)
        }
        InstructionOperation::ElementStoreManaged => {
            super::handle_element_store_managed(state, block, pc)
        }
        InstructionOperation::ElementStoreRaw => super::handle_element_store_raw(state, block, pc),
        InstructionOperation::ElementStoreStack => {
            super::handle_element_store_stack(state, block, pc)
        }
        InstructionOperation::EqConstInt => super::handle_eq_const_int(state, block, pc),
        InstructionOperation::EqInt => super::handle_eq_int(state, block, pc),
        InstructionOperation::FieldAddr => super::handle_field_addr(state, block, pc),
        InstructionOperation::FieldAddrAggregate => {
            super::handle_field_addr_aggregate(state, block, pc)
        }
        InstructionOperation::FieldAddrGlobal => super::handle_field_addr_global(state, block, pc),
        InstructionOperation::FieldAddrManaged => {
            super::handle_field_addr_managed(state, block, pc)
        }
        InstructionOperation::FieldAddrRaw => super::handle_field_addr_raw(state, block, pc),
        InstructionOperation::FieldAddrStack => super::handle_field_addr_stack(state, block, pc),
        InstructionOperation::FieldGet => super::handle_field_get(state, block, pc),
        InstructionOperation::FieldGetInline => super::handle_field_get_inline(state, block, pc),
        InstructionOperation::FieldLoad => super::handle_field_load(state, block, pc),
        InstructionOperation::FieldLoadAggregate => {
            super::handle_field_load_aggregate(state, block, pc)
        }
        InstructionOperation::FieldLoadGlobal => super::handle_field_load_global(state, block, pc),
        InstructionOperation::FieldLoadManaged => {
            super::handle_field_load_managed(state, block, pc)
        }
        InstructionOperation::FieldLoadRaw => super::handle_field_load_raw(state, block, pc),
        InstructionOperation::FieldLoadStack => super::handle_field_load_stack(state, block, pc),
        InstructionOperation::FieldSet => super::handle_field_set(state, block, pc),
        InstructionOperation::FieldStore => super::handle_field_store(state, block, pc),
        InstructionOperation::FieldStoreAggregate => {
            super::handle_field_store_aggregate(state, block, pc)
        }
        InstructionOperation::FieldStoreGlobal => {
            super::handle_field_store_global(state, block, pc)
        }
        InstructionOperation::FieldStoreInline => {
            super::handle_field_store_inline(state, block, pc)
        }
        InstructionOperation::FieldStoreManaged => {
            super::handle_field_store_managed(state, block, pc)
        }
        InstructionOperation::FieldStoreRaw => super::handle_field_store_raw(state, block, pc),
        InstructionOperation::FieldStoreStack => super::handle_field_store_stack(state, block, pc),
        InstructionOperation::FunctionAddr => super::handle_function_addr(state, block, pc),
        InstructionOperation::FunctionEnv => super::handle_function_env(state, block, pc),
        InstructionOperation::GeConstInt => super::handle_ge_const_int(state, block, pc),
        InstructionOperation::GeConstUint => super::handle_ge_const_uint(state, block, pc),
        InstructionOperation::GeInt => super::handle_ge_int(state, block, pc),
        InstructionOperation::GeUint => super::handle_ge_uint(state, block, pc),
        InstructionOperation::GlobalAddr => super::handle_global_addr(state, block, pc),
        InstructionOperation::GlobalConst => super::handle_global_const(state, block, pc),
        InstructionOperation::GlobalLoad => super::handle_global_load(state, block, pc),
        InstructionOperation::GlobalStore => super::handle_global_store(state, block, pc),
        InstructionOperation::GtConstInt => super::handle_gt_const_int(state, block, pc),
        InstructionOperation::GtConstUint => super::handle_gt_const_uint(state, block, pc),
        InstructionOperation::GtInt => super::handle_gt_int(state, block, pc),
        InstructionOperation::GtUint => super::handle_gt_uint(state, block, pc),
        InstructionOperation::Intrinsic => super::handle_intrinsic(state, block, pc),
        InstructionOperation::Jump => super::handle_jump(state, block, pc),
        InstructionOperation::LeConstInt => super::handle_le_const_int(state, block, pc),
        InstructionOperation::LeConstUint => super::handle_le_const_uint(state, block, pc),
        InstructionOperation::LeInt => super::handle_le_int(state, block, pc),
        InstructionOperation::LeUint => super::handle_le_uint(state, block, pc),
        InstructionOperation::Load => super::handle_load(state, block, pc),
        InstructionOperation::LoadGlobal => super::handle_load_global(state, block, pc),
        InstructionOperation::LoadLocal => super::handle_load_local(state, block, pc),
        InstructionOperation::LoadManaged => super::handle_load_managed(state, block, pc),
        InstructionOperation::LoadRaw => super::handle_load_raw(state, block, pc),
        InstructionOperation::LoadStack => super::handle_load_stack(state, block, pc),
        InstructionOperation::LocalAddr => super::handle_local_addr(state, block, pc),
        InstructionOperation::LocalGet => super::handle_local_get(state, block, pc),
        InstructionOperation::LocalSet => super::handle_local_set(state, block, pc),
        InstructionOperation::LtConstInt => super::handle_lt_const_int(state, block, pc),
        InstructionOperation::LtConstUint => super::handle_lt_const_uint(state, block, pc),
        InstructionOperation::LtInt => super::handle_lt_int(state, block, pc),
        InstructionOperation::LtUint => super::handle_lt_uint(state, block, pc),
        InstructionOperation::ManagedAlloc => super::handle_managed_alloc(state, block, pc),
        InstructionOperation::ManagedAllocArray => {
            super::handle_managed_alloc_array(state, block, pc)
        }
        InstructionOperation::MulConstInt => super::handle_mul_const_int(state, block, pc),
        InstructionOperation::MulConstUint => super::handle_mul_const_uint(state, block, pc),
        InstructionOperation::MulInt => super::handle_mul_int(state, block, pc),
        InstructionOperation::MulUint => super::handle_mul_uint(state, block, pc),
        InstructionOperation::NeConstInt => super::handle_ne_const_int(state, block, pc),
        InstructionOperation::NeInt => super::handle_ne_int(state, block, pc),
        InstructionOperation::OrInt => super::handle_or_int(state, block, pc),
        InstructionOperation::OrUint => super::handle_or_uint(state, block, pc),
        InstructionOperation::RawAlloc => super::handle_raw_alloc(state, block, pc),
        InstructionOperation::RawDrop => super::handle_raw_drop(state, block, pc),
        InstructionOperation::RawFree => super::handle_raw_free(state, block, pc),
        InstructionOperation::Return => super::handle_return(state, block, pc),
        InstructionOperation::Select => super::handle_select(state, block, pc),
        InstructionOperation::ShlInt => super::handle_shl_int(state, block, pc),
        InstructionOperation::ShlUint => super::handle_shl_uint(state, block, pc),
        InstructionOperation::ShrInt => super::handle_shr_int(state, block, pc),
        InstructionOperation::ShrUint => super::handle_shr_uint(state, block, pc),
        InstructionOperation::StackAlloc => super::handle_stack_alloc(state, block, pc),
        InstructionOperation::StackDrop => super::handle_stack_drop(state, block, pc),
        InstructionOperation::Store => super::handle_store(state, block, pc),
        InstructionOperation::StoreGlobal => super::handle_store_global(state, block, pc),
        InstructionOperation::StoreLocal => super::handle_store_local(state, block, pc),
        InstructionOperation::StoreManaged => super::handle_store_managed(state, block, pc),
        InstructionOperation::StoreRaw => super::handle_store_raw(state, block, pc),
        InstructionOperation::StoreStack => super::handle_store_stack(state, block, pc),
        InstructionOperation::SubConstInt => super::handle_sub_const_int(state, block, pc),
        InstructionOperation::SubConstUint => super::handle_sub_const_uint(state, block, pc),
        InstructionOperation::SubInt => super::handle_sub_int(state, block, pc),
        InstructionOperation::SubUint => super::handle_sub_uint(state, block, pc),
        InstructionOperation::Switch => super::handle_switch(state, block, pc),
        InstructionOperation::SwitchInt => super::handle_switch_int(state, block, pc),
        InstructionOperation::SwitchTable => super::handle_switch_table(state, block, pc),
        InstructionOperation::SwitchTableInt => super::handle_switch_table_int(state, block, pc),
        InstructionOperation::TailCall => super::handle_tail_call(state, block, pc),
        InstructionOperation::TailCallIndirect => {
            super::handle_tail_call_indirect(state, block, pc)
        }
        InstructionOperation::TailCallInterface => {
            super::handle_tail_call_interface(state, block, pc)
        }
        InstructionOperation::TailCallSelf => super::handle_tail_call_self(state, block, pc),
        InstructionOperation::TailCallVirtual => super::handle_tail_call_virtual(state, block, pc),
        InstructionOperation::TensorBroadcast => super::handle_tensor_broadcast(state, block, pc),
        InstructionOperation::TensorCast => super::handle_tensor_cast(state, block, pc),
        InstructionOperation::TensorCompare => super::handle_tensor_compare(state, block, pc),
        InstructionOperation::TensorConcat => super::handle_tensor_concat(state, block, pc),
        InstructionOperation::TensorConvert => super::handle_tensor_convert(state, block, pc),
        InstructionOperation::TensorConvolution => {
            super::handle_tensor_convolution(state, block, pc)
        }
        InstructionOperation::TensorCopy => super::handle_tensor_copy(state, block, pc),
        InstructionOperation::TensorDot => super::handle_tensor_dot(state, block, pc),
        InstructionOperation::TensorFill => super::handle_tensor_fill(state, block, pc),
        InstructionOperation::TensorGather => super::handle_tensor_gather(state, block, pc),
        InstructionOperation::TensorLoad => super::handle_tensor_load(state, block, pc),
        InstructionOperation::TensorPad => super::handle_tensor_pad(state, block, pc),
        InstructionOperation::TensorReduce => super::handle_tensor_reduce(state, block, pc),
        InstructionOperation::TensorReshape => super::handle_tensor_reshape(state, block, pc),
        InstructionOperation::TensorScatter => super::handle_tensor_scatter(state, block, pc),
        InstructionOperation::TensorSelect => super::handle_tensor_select(state, block, pc),
        InstructionOperation::TensorSlice => super::handle_tensor_slice(state, block, pc),
        InstructionOperation::TensorStore => super::handle_tensor_store(state, block, pc),
        InstructionOperation::TensorTranspose => super::handle_tensor_transpose(state, block, pc),
        InstructionOperation::TensorView => super::handle_tensor_view(state, block, pc),
        InstructionOperation::Trap => super::handle_trap(state, block, pc),
        InstructionOperation::Unary => super::handle_unary(state, block, pc),
        InstructionOperation::UnaryBool => super::handle_unary_bool(state, block, pc),
        InstructionOperation::UnaryElementwise => super::handle_unary_elementwise(state, block, pc),
        InstructionOperation::UnaryFloat32 => super::handle_unary_float32(state, block, pc),
        InstructionOperation::UnaryFloat64 => super::handle_unary_float64(state, block, pc),
        InstructionOperation::UnaryInt => super::handle_unary_int(state, block, pc),
        InstructionOperation::UnaryUint => super::handle_unary_uint(state, block, pc),
        InstructionOperation::Unreachable => super::handle_unreachable(state, block, pc),
        InstructionOperation::VectorCompare => super::handle_vector_compare(state, block, pc),
        InstructionOperation::VectorConvert => super::handle_vector_convert(state, block, pc),
        InstructionOperation::VectorExtract => super::handle_vector_extract(state, block, pc),
        InstructionOperation::VectorInsert => super::handle_vector_insert(state, block, pc),
        InstructionOperation::VectorReduce => super::handle_vector_reduce(state, block, pc),
        InstructionOperation::VectorSelect => super::handle_vector_select(state, block, pc),
        InstructionOperation::VectorShuffle => super::handle_vector_shuffle(state, block, pc),
        InstructionOperation::VectorSplat => super::handle_vector_splat(state, block, pc),
        InstructionOperation::XorInt => super::handle_xor_int(state, block, pc),
        InstructionOperation::XorUint => super::handle_xor_uint(state, block, pc),
        InstructionOperation::Yield => super::handle_yield(state, block, pc),
    }
}

// advance to the next instruction in one lowered block
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        $state.maybe_profile_instruction(&$block[$pc]);
        let next_pc = $pc + 1;
        become $crate::interpreter::execute::dispatch_instruction($state, $block, next_pc)
    }};
}

pub(crate) use next;
