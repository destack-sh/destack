#![allow(elided_lifetimes_in_paths)]

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::Value;

use super::threaded::{
    ArgumentRange, ControlFlow, ThreadedInstruction, ThreadedInstructionData, ThreadedState,
    is_invalid_value,
};
use super::{instruction, operator};

// helper macro: do work, then become next handler
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        let next_pc = $pc + 1;
        become ($block[next_pc].handler)($state, $block, next_pc)
    }};
}

/// Build a local id from a raw value.
#[inline]
fn local_id(raw: u32) -> mir::LocalNodeId<mir::Local> {
    mir::LocalNodeId::new(raw)
}

/// Build a global id from a raw value.
#[inline]
fn global_id(raw: u32) -> mir::LocalNodeId<mir::Global> {
    mir::LocalNodeId::new(raw)
}

/// Build a type id from a raw value.
#[inline]
fn type_id(raw: u32) -> mir::LocalNodeId<mir::Type> {
    mir::LocalNodeId::new(raw)
}

/// Collect argument values into a smallvec.
#[inline]
fn collect_values(state: &mut ThreadedState, arguments: ArgumentRange) -> SmallVec<[Value; 8]> {
    // load argument slice
    let argument_slice = state.argument_slice(arguments);
    let mut args = SmallVec::with_capacity(argument_slice.len());

    // resolve argument values
    for arg in argument_slice {
        let value = state.get(*arg);
        args.push(value);
    }

    // return argument values
    args
}

/// Handle constant load.
pub(super) fn handle_const(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Const { dest, value } = &block[pc].data else {
        unreachable!()
    };

    // write value
    state.set(*dest, *value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle binary operation.
pub(super) fn handle_binary(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle signed integer binary operation.
pub(super) fn handle_binary_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_int(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unsigned integer binary operation.
pub(super) fn handle_binary_uint(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_uint(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float32 binary operation.
pub(super) fn handle_binary_float32(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_float32(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float64 binary operation.
pub(super) fn handle_binary_float64(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_float64(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle boolean binary operation.
pub(super) fn handle_binary_bool(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_bool(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unary operation.
pub(super) fn handle_unary(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle signed integer unary operation.
pub(super) fn handle_unary_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_int(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unsigned integer unary operation.
pub(super) fn handle_unary_uint(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_uint(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float32 unary operation.
pub(super) fn handle_unary_float32(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_float32(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float64 unary operation.
pub(super) fn handle_unary_float64(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_float64(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle boolean unary operation.
pub(super) fn handle_unary_bool(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_bool(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle cast operation.
pub(super) fn handle_cast(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Cast {
        dest,
        op,
        arg,
        to_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute cast
    let cast_type = type_id(*to_type);
    let result = match operator::execute_cast(&state.interpreter.tree, *op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle function call (returns to trampoline).
pub(super) fn handle_call(
    _state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Call {
        dest,
        function,
        arguments,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // return control to trampoline
    ControlFlow::Call {
        function: *function,
        destination: *dest,
        arguments: *arguments,
        copies: Some(*copies),
        resume_pc: pc + 1,
    }
}

/// Handle indirect call (returns to trampoline).
pub(super) fn handle_call_indirect(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::CallIndirect {
        dest,
        callee,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // extract function pointer
    let function = match callee_val.as_function_pointer() {
        Some(f) => f.id,
        None => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "function_pointer".to_string(),
                actual: format!("{callee_val:?}"),
            });
        }
    };

    // return control to trampoline
    ControlFlow::Call {
        function,
        destination: *dest,
        arguments: *arguments,
        copies: None,
        resume_pc: pc + 1,
    }
}

/// Handle local variable load.
pub(super) fn handle_local_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalGet { dest, local } = &block[pc].data else {
        unreachable!()
    };

    // load local value
    let value = state.get_local(local_id(*local));

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local variable store.
pub(super) fn handle_local_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalSet { local, value } = &block[pc].data else {
        unreachable!()
    };

    // load value
    let val = state.get(*value);

    // store local value
    state.set_local(local_id(*local), val);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global address.
pub(super) fn handle_global_addr(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalAddr { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // write global pointer
    state.set(*dest, Value::global_pointer(global_id(*global)));

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global constant load.
pub(super) fn handle_global_const(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalConst { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // load global value
    let global_id = global_id(*global);
    let value = match state.interpreter.globals.get(global_id).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: global_id }),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer load.
pub(super) fn handle_load(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from pointer
    let value = match instruction::load_from_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer store.
pub(super) fn handle_store(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store { pointer, value } = &block[pc].data else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // write through pointer
    if let Err(e) = instruction::store_to_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field get.
pub(super) fn handle_field_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldGet {
        dest,
        aggregate,
        index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // load field value
    let value = match instruction::get_field(state, agg, *index) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field set.
pub(super) fn handle_field_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldSet {
        dest,
        aggregate,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // write field
    let result = match instruction::set_field(state, agg, *index, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element get.
pub(super) fn handle_element_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementGet { dest, array, index } = &block[pc].data else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let value = match instruction::get_element(state, arr, idx_val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element set.
pub(super) fn handle_element_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementSet {
        dest,
        array,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // write element
    let result = match instruction::set_element(state, arr, idx_val, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed allocation.
pub(super) fn handle_managed_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ManagedAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.managed_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // allocate heap cell
    let handle = state.interpreter.managed_heap.allocate();
    state.interpreter.statistics.heap_allocations += 1;
    state.set(*dest, Value::managed_reference(handle));

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed array allocation.
pub(super) fn handle_managed_alloc_array(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ManagedAllocArray { dest, length } = &block[pc].data else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.managed_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // resolve array length
    let len_val = state.get(*length);
    let length = len_val.as_uint().unwrap_or(0) as usize;

    // allocate heap cell with slots
    let handle = state.interpreter.managed_heap.allocate_with_slots(length);
    state.interpreter.statistics.heap_allocations += 1;
    state.set(*dest, Value::managed_reference(handle));

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw allocation.
pub(super) fn handle_raw_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.raw_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // allocate raw heap cell
    let ptr = state.interpreter.raw_heap.allocate();
    state.interpreter.statistics.heap_allocations += 1;
    state.set(*dest, Value::raw_pointer(ptr));

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw free.
pub(super) fn handle_raw_free(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawFree { pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // accept raw pointer values
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        if !state.interpreter.raw_heap.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    }
    // otherwise report type mismatch
    else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack allocation.
pub(super) fn handle_stack_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::StackAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    // NOTE #Incomplete: stack allocation requires proper layout sizing
    let frame_index = state.frame_index;
    let slot = state.current_frame_mut().allocate_stack_cell();
    let sp = crate::memory::StackPointer::new(frame_index, slot);
    state.set(*dest, Value::stack_pointer(sp));

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle intrinsic call.
pub(super) fn handle_intrinsic(
    state: &mut ThreadedState<'_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Intrinsic {
        dest,
        intrinsic,
        arguments,
        ordering,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve arguments
    let args = collect_values(state, *arguments);

    // execute intrinsic
    match state
        .interpreter
        .execute_intrinsic_resolved(*intrinsic, args.as_slice(), *ordering)
    {
        // store result and continue
        Ok(result) => {
            if !is_invalid_value(*dest) {
                state.set(*dest, result);
            }
            next!(state, block, pc)
        }
        // return runtime error
        Err(e) => ControlFlow::Error(e.error),
    }
}

/// Handle return (exits tail-call chain).
pub(super) fn handle_return(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Return { value } = &block[pc].data else {
        unreachable!()
    };

    // resolve return value
    let return_value = if is_invalid_value(*value) {
        Value::VOID
    } else {
        state.get(*value)
    };

    // return to caller
    ControlFlow::Return(return_value)
}

/// Handle unconditional jump (exits tail-call chain).
pub(super) fn handle_jump(
    _state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Jump { target, copies } = &block[pc].data else {
        unreachable!()
    };

    // return jump control
    ControlFlow::Jump {
        block: *target,
        copies: *copies,
    }
}

/// Handle conditional branch (exits tail-call chain).
pub(super) fn handle_branch(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Branch {
        condition,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.is_truthy();

    // update branch statistics
    state.interpreter.statistics.branches += 1;

    // handle truthy branch
    if is_truthy {
        // forward then copies
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    }
    // otherwise jump to else target
    else {
        // forward else copies
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle boolean branch (exits tail-call chain).
pub(super) fn handle_branch_bool(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Branch {
        condition,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.raw_data() != 0;

    // update branch statistics
    state.interpreter.statistics.branches += 1;

    // handle truthy branch
    if is_truthy {
        // forward then copies
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    }
    // otherwise jump to else target
    else {
        // forward else copies
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle switch (exits tail-call chain).
pub(super) fn handle_switch(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Switch {
        value,
        cases,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.as_int().unwrap_or(0);

    // update branch statistics
    state.interpreter.statistics.branches += 1;

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return ControlFlow::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // forward default copies
    // return default jump
    ControlFlow::Jump {
        block: *default_target,
        copies: *default_copies,
    }
}

/// Handle integer switch (exits tail-call chain).
pub(super) fn handle_switch_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Switch {
        value,
        cases,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.raw_data() as i64;

    // update branch statistics
    state.interpreter.statistics.branches += 1;

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return ControlFlow::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // forward default copies
    // return default jump
    ControlFlow::Jump {
        block: *default_target,
        copies: *default_copies,
    }
}

/// Handle unreachable (errors).
pub(super) fn handle_unreachable(
    _state: &mut ThreadedState,
    _block: &[ThreadedInstruction],
    _pc: usize,
) -> ControlFlow {
    // return unreachable error
    ControlFlow::Error(Error::Unreachable)
}

/// Handle unsupported instructions (errors).
pub(super) fn handle_unsupported(
    _state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unsupported { name } = &block[pc].data else {
        unreachable!()
    };

    // return unsupported error
    ControlFlow::Error(Error::UnsupportedInstruction {
        name: (*name).to_string(),
    })
}
