use crate::executable::{Instruction, InstructionOperation, Transfer};
use crate::interpreter::StepState;

/// Step one lowered instruction through the interpreter machine surface.
pub(crate) fn step_instruction(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    match block[pc].operation {
        InstructionOperation::AddConstInt => super::step_add_const_int(state, block, pc),
        InstructionOperation::AddConstUint => super::step_add_const_uint(state, block, pc),
        InstructionOperation::AddInt => super::step_add_int(state, block, pc),
        InstructionOperation::AddUint => super::step_add_uint(state, block, pc),
        InstructionOperation::Composite => super::step_composite(state, block, pc),
        InstructionOperation::AndInt => super::step_and_int(state, block, pc),
        InstructionOperation::AndUint => super::step_and_uint(state, block, pc),
        InstructionOperation::Assume => super::step_assume(state, block, pc),
        InstructionOperation::AtomicCompareExchange => {
            super::step_atomic_compare_exchange(state, block, pc)
        }
        InstructionOperation::AtomicFence => super::step_atomic_fence(state, block, pc),
        InstructionOperation::AtomicLoad => super::step_atomic_load(state, block, pc),
        InstructionOperation::AtomicRmw => super::step_atomic_rmw(state, block, pc),
        InstructionOperation::AtomicStore => super::step_atomic_store(state, block, pc),
        InstructionOperation::Barrier => super::step_barrier(state, block, pc),
        InstructionOperation::Binary => super::step_binary(state, block, pc),
        InstructionOperation::BinaryBool => super::step_binary_bool(state, block, pc),
        InstructionOperation::BinaryConstRight => super::step_binary_const_right(state, block, pc),
        InstructionOperation::BinaryElementwise => super::step_binary_elementwise(state, block, pc),
        InstructionOperation::BinaryFloat32 => super::step_binary_float32(state, block, pc),
        InstructionOperation::BinaryFloat64 => super::step_binary_float64(state, block, pc),
        InstructionOperation::BinaryInt => super::step_binary_int(state, block, pc),
        InstructionOperation::BinaryUint => super::step_binary_uint(state, block, pc),
        InstructionOperation::Branch => super::step_branch(state, block, pc),
        InstructionOperation::BranchBool => super::step_branch_bool(state, block, pc),
        InstructionOperation::Check => super::step_check(state, block, pc),
        InstructionOperation::Call => super::step_call(state, block, pc),
        InstructionOperation::CallBranch => super::step_call_branch(state, block, pc),
        InstructionOperation::CallIndirect => super::step_call_indirect(state, block, pc),
        InstructionOperation::CallIndirectBranch => {
            super::step_call_indirect_branch(state, block, pc)
        }
        InstructionOperation::CallInterface => super::step_call_interface(state, block, pc),
        InstructionOperation::CallInterfaceBranch => {
            super::step_call_interface_branch(state, block, pc)
        }
        InstructionOperation::CallVirtual => super::step_call_virtual(state, block, pc),
        InstructionOperation::CallVirtualBranch => {
            super::step_call_virtual_branch(state, block, pc)
        }
        InstructionOperation::Cast => super::step_cast(state, block, pc),
        InstructionOperation::CompareAndBranch => super::step_compare_and_branch(state, block, pc),
        InstructionOperation::CompareAndBranchConst => {
            super::step_compare_and_branch_const(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstFloat => {
            super::step_compare_and_branch_const_float(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstInt => {
            super::step_compare_and_branch_const_int(state, block, pc)
        }
        InstructionOperation::CompareAndBranchConstUint => {
            super::step_compare_and_branch_const_uint(state, block, pc)
        }
        InstructionOperation::CompareAndBranchFloat => {
            super::step_compare_and_branch_float(state, block, pc)
        }
        InstructionOperation::CompareAndBranchInt => {
            super::step_compare_and_branch_int(state, block, pc)
        }
        InstructionOperation::CompareAndBranchUint => {
            super::step_compare_and_branch_uint(state, block, pc)
        }
        InstructionOperation::Const => super::step_const(state, block, pc),
        InstructionOperation::Copy => super::step_copy(state, block, pc),
        InstructionOperation::IndexSelect => super::step_index_select(state, block, pc),
        InstructionOperation::SelectByIndex => super::step_select_by_index(state, block, pc),
        InstructionOperation::ElementAddr => super::step_element_addr(state, block, pc),
        InstructionOperation::ElementAddrComposite => {
            super::step_element_addr_composite(state, block, pc)
        }
        InstructionOperation::ElementAddrGlobal => {
            super::step_element_addr_global(state, block, pc)
        }
        InstructionOperation::ElementAddrManaged => {
            super::step_element_addr_managed(state, block, pc)
        }
        InstructionOperation::ElementAddrRaw => super::step_element_addr_raw(state, block, pc),
        InstructionOperation::ElementAddrStack => super::step_element_addr_stack(state, block, pc),
        InstructionOperation::ElementGet => super::step_element_get(state, block, pc),
        InstructionOperation::ElementLoad => super::step_element_load(state, block, pc),
        InstructionOperation::ElementLoadComposite => {
            super::step_element_load_composite(state, block, pc)
        }
        InstructionOperation::ElementLoadGlobal => {
            super::step_element_load_global(state, block, pc)
        }
        InstructionOperation::ElementLoadManaged => {
            super::step_element_load_managed(state, block, pc)
        }
        InstructionOperation::ElementLoadRaw => super::step_element_load_raw(state, block, pc),
        InstructionOperation::ElementLoadStack => super::step_element_load_stack(state, block, pc),
        InstructionOperation::ElementSet => super::step_element_set(state, block, pc),
        InstructionOperation::ElementStore => super::step_element_store(state, block, pc),
        InstructionOperation::ElementStoreComposite => {
            super::step_element_store_composite(state, block, pc)
        }
        InstructionOperation::ElementStoreGlobal => {
            super::step_element_store_global(state, block, pc)
        }
        InstructionOperation::ElementStoreManaged => {
            super::step_element_store_managed(state, block, pc)
        }
        InstructionOperation::ElementStoreRaw => super::step_element_store_raw(state, block, pc),
        InstructionOperation::ElementStoreStack => {
            super::step_element_store_stack(state, block, pc)
        }
        InstructionOperation::EqConstInt => super::step_eq_const_int(state, block, pc),
        InstructionOperation::EqInt => super::step_eq_int(state, block, pc),
        InstructionOperation::FieldAddr => super::step_field_addr(state, block, pc),
        InstructionOperation::FieldAddrComposite => {
            super::step_field_addr_composite(state, block, pc)
        }
        InstructionOperation::FieldAddrGlobal => super::step_field_addr_global(state, block, pc),
        InstructionOperation::FieldAddrManaged => super::step_field_addr_managed(state, block, pc),
        InstructionOperation::FieldAddrRaw => super::step_field_addr_raw(state, block, pc),
        InstructionOperation::FieldAddrStack => super::step_field_addr_stack(state, block, pc),
        InstructionOperation::FieldGet => super::step_field_get(state, block, pc),
        InstructionOperation::FieldGetInline => super::step_field_get_inline(state, block, pc),
        InstructionOperation::FieldLoad => super::step_field_load(state, block, pc),
        InstructionOperation::FieldLoadComposite => {
            super::step_field_load_composite(state, block, pc)
        }
        InstructionOperation::FieldLoadGlobal => super::step_field_load_global(state, block, pc),
        InstructionOperation::FieldLoadManaged => super::step_field_load_managed(state, block, pc),
        InstructionOperation::FieldLoadRaw => super::step_field_load_raw(state, block, pc),
        InstructionOperation::FieldLoadStack => super::step_field_load_stack(state, block, pc),
        InstructionOperation::FieldSet => super::step_field_set(state, block, pc),
        InstructionOperation::FieldStore => super::step_field_store(state, block, pc),
        InstructionOperation::FieldStoreComposite => {
            super::step_field_store_composite(state, block, pc)
        }
        InstructionOperation::FieldStoreGlobal => super::step_field_store_global(state, block, pc),
        InstructionOperation::FieldStoreInline => super::step_field_store_inline(state, block, pc),
        InstructionOperation::FieldStoreManaged => {
            super::step_field_store_managed(state, block, pc)
        }
        InstructionOperation::FieldStoreRaw => super::step_field_store_raw(state, block, pc),
        InstructionOperation::FieldStoreStack => super::step_field_store_stack(state, block, pc),
        InstructionOperation::FunctionAddr => super::step_function_addr(state, block, pc),
        InstructionOperation::FunctionBind => super::step_function_bind(state, block, pc),
        InstructionOperation::FunctionEnvironment => {
            super::step_function_environment(state, block, pc)
        }
        InstructionOperation::GeConstInt => super::step_ge_const_int(state, block, pc),
        InstructionOperation::GeConstUint => super::step_ge_const_uint(state, block, pc),
        InstructionOperation::GeInt => super::step_ge_int(state, block, pc),
        InstructionOperation::GeUint => super::step_ge_uint(state, block, pc),
        InstructionOperation::GlobalAddr => super::step_global_addr(state, block, pc),
        InstructionOperation::GlobalConst => super::step_global_const(state, block, pc),
        InstructionOperation::GlobalLoad => super::step_global_load(state, block, pc),
        InstructionOperation::GlobalStore => super::step_global_store(state, block, pc),
        InstructionOperation::GtConstInt => super::step_gt_const_int(state, block, pc),
        InstructionOperation::GtConstUint => super::step_gt_const_uint(state, block, pc),
        InstructionOperation::GtInt => super::step_gt_int(state, block, pc),
        InstructionOperation::GtUint => super::step_gt_uint(state, block, pc),
        InstructionOperation::Intrinsic => super::step_intrinsic(state, block, pc),
        InstructionOperation::Jump => super::step_jump(state, block, pc),
        InstructionOperation::LeConstInt => super::step_le_const_int(state, block, pc),
        InstructionOperation::LeConstUint => super::step_le_const_uint(state, block, pc),
        InstructionOperation::LeInt => super::step_le_int(state, block, pc),
        InstructionOperation::LeUint => super::step_le_uint(state, block, pc),
        InstructionOperation::Load => super::step_load(state, block, pc),
        InstructionOperation::LoadGlobal => super::step_load_global(state, block, pc),
        InstructionOperation::LoadLocal => super::step_load_local(state, block, pc),
        InstructionOperation::LoadManaged => super::step_load_managed(state, block, pc),
        InstructionOperation::LoadRaw => super::step_load_raw(state, block, pc),
        InstructionOperation::LoadStack => super::step_load_stack(state, block, pc),
        InstructionOperation::LocalAddr => super::step_local_addr(state, block, pc),
        InstructionOperation::LocalGet => super::step_local_get(state, block, pc),
        InstructionOperation::LocalSet => super::step_local_set(state, block, pc),
        InstructionOperation::LtConstInt => super::step_lt_const_int(state, block, pc),
        InstructionOperation::LtConstUint => super::step_lt_const_uint(state, block, pc),
        InstructionOperation::LtInt => super::step_lt_int(state, block, pc),
        InstructionOperation::LtUint => super::step_lt_uint(state, block, pc),
        InstructionOperation::ManagedAlloc => super::step_managed_alloc(state, block, pc),
        InstructionOperation::ManagedAllocArray => {
            super::step_managed_alloc_array(state, block, pc)
        }
        InstructionOperation::MulConstInt => super::step_mul_const_int(state, block, pc),
        InstructionOperation::MulConstUint => super::step_mul_const_uint(state, block, pc),
        InstructionOperation::MulInt => super::step_mul_int(state, block, pc),
        InstructionOperation::MulUint => super::step_mul_uint(state, block, pc),
        InstructionOperation::NeConstInt => super::step_ne_const_int(state, block, pc),
        InstructionOperation::NeInt => super::step_ne_int(state, block, pc),
        InstructionOperation::OrInt => super::step_or_int(state, block, pc),
        InstructionOperation::OrUint => super::step_or_uint(state, block, pc),
        InstructionOperation::RawAlloc => super::step_raw_alloc(state, block, pc),
        InstructionOperation::Dispose => super::step_dispose(state, block, pc),
        InstructionOperation::AsyncDispose => super::step_async_dispose(state, block, pc),
        InstructionOperation::Drop => super::step_drop(state, block, pc),
        InstructionOperation::AsyncDrop => super::step_async_drop(state, block, pc),
        InstructionOperation::RawFree => super::step_raw_free(state, block, pc),
        InstructionOperation::Return => super::step_return(state, block, pc),
        InstructionOperation::Select => super::step_select(state, block, pc),
        InstructionOperation::ShlInt => super::step_shl_int(state, block, pc),
        InstructionOperation::ShlUint => super::step_shl_uint(state, block, pc),
        InstructionOperation::ShrInt => super::step_shr_int(state, block, pc),
        InstructionOperation::ShrUint => super::step_shr_uint(state, block, pc),
        InstructionOperation::StackAlloc => super::step_stack_alloc(state, block, pc),
        InstructionOperation::Store => super::step_store(state, block, pc),
        InstructionOperation::StoreGlobal => super::step_store_global(state, block, pc),
        InstructionOperation::StoreLocal => super::step_store_local(state, block, pc),
        InstructionOperation::StoreManaged => super::step_store_managed(state, block, pc),
        InstructionOperation::StoreRaw => super::step_store_raw(state, block, pc),
        InstructionOperation::StoreStack => super::step_store_stack(state, block, pc),
        InstructionOperation::SubConstInt => super::step_sub_const_int(state, block, pc),
        InstructionOperation::SubConstUint => super::step_sub_const_uint(state, block, pc),
        InstructionOperation::SubInt => super::step_sub_int(state, block, pc),
        InstructionOperation::SubUint => super::step_sub_uint(state, block, pc),
        InstructionOperation::Switch => super::step_switch(state, block, pc),
        InstructionOperation::SwitchInt => super::step_switch_int(state, block, pc),
        InstructionOperation::SwitchTable => super::step_switch_table(state, block, pc),
        InstructionOperation::SwitchTableInt => super::step_switch_table_int(state, block, pc),
        InstructionOperation::TailCall => super::step_tail_call(state, block, pc),
        InstructionOperation::TailCallIndirect => super::step_tail_call_indirect(state, block, pc),
        InstructionOperation::TailCallInterface => {
            super::step_tail_call_interface(state, block, pc)
        }
        InstructionOperation::TailCallSelf => super::step_tail_call_self(state, block, pc),
        InstructionOperation::TailCallVirtual => super::step_tail_call_virtual(state, block, pc),
        InstructionOperation::TensorBroadcast => super::step_tensor_broadcast(state, block, pc),
        InstructionOperation::TensorCast => super::step_tensor_cast(state, block, pc),
        InstructionOperation::TensorCompare => super::step_tensor_compare(state, block, pc),
        InstructionOperation::TensorConcat => super::step_tensor_concat(state, block, pc),
        InstructionOperation::TensorConvert => super::step_tensor_convert(state, block, pc),
        InstructionOperation::TensorConvolution => super::step_tensor_convolution(state, block, pc),
        InstructionOperation::TensorCopy => super::step_tensor_copy(state, block, pc),
        InstructionOperation::TensorDot => super::step_tensor_dot(state, block, pc),
        InstructionOperation::TensorFill => super::step_tensor_fill(state, block, pc),
        InstructionOperation::TensorGather => super::step_tensor_gather(state, block, pc),
        InstructionOperation::TensorLoad => super::step_tensor_load(state, block, pc),
        InstructionOperation::TensorPad => super::step_tensor_pad(state, block, pc),
        InstructionOperation::TensorReduce => super::step_tensor_reduce(state, block, pc),
        InstructionOperation::TensorReshape => super::step_tensor_reshape(state, block, pc),
        InstructionOperation::TensorScatter => super::step_tensor_scatter(state, block, pc),
        InstructionOperation::TensorSelect => super::step_tensor_select(state, block, pc),
        InstructionOperation::TensorSlice => super::step_tensor_slice(state, block, pc),
        InstructionOperation::TensorStore => super::step_tensor_store(state, block, pc),
        InstructionOperation::TensorTranspose => super::step_tensor_transpose(state, block, pc),
        InstructionOperation::TensorView => super::step_tensor_view(state, block, pc),
        InstructionOperation::Trap => super::step_trap(state, block, pc),
        InstructionOperation::Throw => super::step_throw(state, block, pc),
        InstructionOperation::Unary => super::step_unary(state, block, pc),
        InstructionOperation::UnaryBool => super::step_unary_bool(state, block, pc),
        InstructionOperation::UnaryElementwise => super::step_unary_elementwise(state, block, pc),
        InstructionOperation::UnaryFloat32 => super::step_unary_float32(state, block, pc),
        InstructionOperation::UnaryFloat64 => super::step_unary_float64(state, block, pc),
        InstructionOperation::UnaryInt => super::step_unary_int(state, block, pc),
        InstructionOperation::UnaryUint => super::step_unary_uint(state, block, pc),
        InstructionOperation::Unreachable => super::step_unreachable(state, block, pc),
        InstructionOperation::VectorCompare => super::step_vector_compare(state, block, pc),
        InstructionOperation::VectorConvert => super::step_vector_convert(state, block, pc),
        InstructionOperation::VectorExtract => super::step_vector_extract(state, block, pc),
        InstructionOperation::VectorInsert => super::step_vector_insert(state, block, pc),
        InstructionOperation::VectorReduce => super::step_vector_reduce(state, block, pc),
        InstructionOperation::VectorSelect => super::step_vector_select(state, block, pc),
        InstructionOperation::VectorShuffle => super::step_vector_shuffle(state, block, pc),
        InstructionOperation::VectorSplat => super::step_vector_splat(state, block, pc),
        InstructionOperation::XorInt => super::step_xor_int(state, block, pc),
        InstructionOperation::XorUint => super::step_xor_uint(state, block, pc),
        InstructionOperation::Yield => super::step_yield(state, block, pc),
    }
}

// advance to the next instruction in one lowered block
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        $state.maybe_profile_instruction(&$block[$pc]);
        let next_pc = $pc + 1;
        become $crate::interpreter::machine::step_instruction($state, $block, next_pc)
    }};
}

pub(crate) use next;
