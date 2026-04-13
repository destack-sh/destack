use destack_engine as engine;

use super::bind::{
    bind_parameters_from_transferred_values, collect_transferred_values_from_copies,
    collect_transferred_values_range, copy_values_with_plan_typed,
};
use super::prelude::*;

const VTABLE_FIELD_INDEX: u32 = 0;
const INTERFACE_ITAB_FIELD_INDEX: u32 = 1;

/// Resolve one receiver field access descriptor from one managed pointee type.
fn receiver_field_access(
    state: &StepState<'_, '_>,
    managed_pointee: mir::LocalNodeId<mir::Type>,
    field_index: u32,
) -> Result<crate::executable::FieldAccess, Error> {
    // load the compiled receiver layout
    let layout = state.layout(managed_pointee)?;
    let field = layout.field(field_index);

    // convert the selected field into one machine access descriptor
    field.map_or_else(
        || {
            Err(Error::TypeMismatch {
                expected: "receiver composite field".to_string(),
                actual: format!("{managed_pointee:?}"),
            })
        },
        |field| {
            let is_scalar = state
                .layout(field.ty)
                .is_ok_and(|layout| layout.is_scalar());

            Ok(crate::executable::FieldAccess {
                value_type: field.ty,
                byte_offset: field.offset,
                byte_len: field.byte_len,
                is_scalar,
            })
        },
    )
}

/// Load a field value from a heap composite receiver.
fn load_receiver_field(
    state: &mut StepState<'_, '_>,
    receiver: Value,
    managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
    field_index: u32,
) -> Result<Value, Error> {
    // resolve based on receiver storage
    match receiver.tag() {
        ValueTag::ManagedReference => {
            let Some(managed_pointee) = managed_pointee else {
                return access::get_field(state, receiver, field_index);
            };

            // resolve the receiver field descriptor and load directly
            let handle = receiver
                .as_managed_reference()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "composite".to_string(),
                    actual: format!("{receiver:?}"),
                })?;
            let field = receiver_field_access(state, managed_pointee, field_index)?;
            access::load_field_managed(state, handle, field, field_index, UNKNOWN_FIELD_COUNT)
        }
        ValueTag::StackPointer => {
            let Some(managed_pointee) = managed_pointee else {
                return access::get_field(state, receiver, field_index);
            };

            let pointer = receiver
                .as_stack_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "composite".to_string(),
                    actual: format!("{receiver:?}"),
                })?;
            let field = receiver_field_access(state, managed_pointee, field_index)?;
            access::load_field_stack(state, pointer, field, field_index, UNKNOWN_FIELD_COUNT)
        }
        _ => Err(Error::TypeMismatch {
            expected: "composite".to_string(),
            actual: format!("{receiver:?}"),
        }),
    }
}

/// Resolve the vtable dispatch target for a virtual call.
fn resolve_virtual_dispatch_target(
    state: &mut StepState<'_, '_>,
    receiver: Value,
    managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
    slot_id: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field(state, receiver, managed_pointee, VTABLE_FIELD_INDEX)?;

    // require a global pointer for the vtable
    let vtable_pointer = vtable_value
        .as_global_pointer()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "global_pointer".to_string(),
            actual: format!("{vtable_value:?}"),
        })?;

    // map the vtable global to a vtable id
    let table_id = state
        .vtable_for_global(vtable_pointer.id)
        .ok_or(Error::InvalidInstruction)?;

    // resolve the vtable slot for the virtual call
    let table = state.tree().metadata.dispatch.vtable(table_id);
    let slot = table
        .entries
        .get(slot_id as usize)
        .ok_or(Error::InvalidInstruction)?;

    // require a method slot
    let mir::VtableEntry::Method { function } = slot else {
        return Err(Error::InvalidInstruction);
    };

    Ok(*function)
}

/// Resolve the itab dispatch target for an interface call.
fn resolve_interface_dispatch_target(
    state: &mut StepState<'_, '_>,
    receiver: Value,
    managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
    slot_id: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the itab id from the interface reference
    let itab_value =
        load_receiver_field(state, receiver, managed_pointee, INTERFACE_ITAB_FIELD_INDEX)?;

    // decode the itab id
    let raw_id = match itab_value.as_uint_with_width() {
        Some((value, _)) => value,
        None => match itab_value.as_int_with_width() {
            Some((value, _)) if value >= 0 => value as u64,
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "itab_id".to_string(),
                    actual: format!("{itab_value:?}"),
                });
            }
        },
    };
    let raw_id = u32::try_from(raw_id).map_err(|_| Error::InvalidInstruction)?;
    let table_id = mir::ItabId::new(raw_id);

    // resolve the itab slot for the interface call
    let table = state.tree().metadata.dispatch.itab(table_id);
    let slot = table
        .entries
        .get(slot_id as usize)
        .ok_or(Error::InvalidInstruction)?;

    // require an interface method slot
    let mir::ItabEntry::Method { target_method, .. } = slot else {
        return Err(Error::InvalidInstruction);
    };

    Ok(*target_method)
}

/// Resolve one indirect callable into function code and environment.
fn resolve_indirect_callable(
    state: &mut StepState<'_, '_>,
    callable: Value,
    callable_type: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Function>, Option<Value>), Error> {
    // dispatch by the actual callee SSA type, not the code signature
    let callable_type = crate::executable::repr_type(state.tree(), callable_type);
    match state.tree().get(callable_type) {
        mir::Type::FunctionPointer { .. } => {
            let function = callable
                .as_function_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "function_pointer".to_string(),
                    actual: format!("{callable:?}"),
                })?;

            Ok((function, None))
        }
        mir::Type::Closure { .. } => {
            let (function_value, environment_value) =
                access::decode_function_value(state, callable)?;
            let function =
                function_value
                    .as_function_pointer()
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "function_pointer".to_string(),
                        actual: format!("{function_value:?}"),
                    })?;

            Ok((function, Some(environment_value)))
        }
        _ => Err(Error::InvalidInstruction),
    }
}

/// Load a function pointer.
pub(crate) fn step_function_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FunctionAddr { dest, function } = &block[pc].data else {
        unreachable!()
    };

    // build function pointer value
    let function_id = mir::LocalNodeId::new(*function);
    let value = Value::function_pointer(function_id);

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Build a callable value from one function and environment.
pub(crate) fn step_function_bind(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let InstructionData::FunctionBind {
        dest,
        function,
        environment,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // build one callable composite in semantic component order
    let function_id = mir::LocalNodeId::new(*function);
    let function_value = Value::function_pointer(function_id);
    let environment_value = state.get(*environment);
    let value =
        match super::value::materialize_composite_by_index(state, *dest, |_state, index, _ty| {
            match index {
                0 => Ok(function_value),
                1 => Ok(environment_value),
                _ => Err(Error::InvalidInstruction),
            }
        }) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Load the function environment pointer for the current frame.
pub(crate) fn step_function_environment(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FunctionEnvironment { dest } = &block[pc].data else {
        unreachable!()
    };

    // load current frame environment
    let environment = state.current_frame_mut().environment;
    if environment == Value::VOID {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // store result
    state.set(*dest, environment);

    // continue to next instruction
    next!(state, block, pc)
}

/// Resolve one lowered direct-call target for the step fast path.
#[inline]
fn resolve_direct_lowered_callee(
    state: &StepState<'_, '_>,
    target: CallTarget,
) -> Option<NonNull<crate::executable::Function>> {
    // only lowered targets can use the direct fast path
    match target {
        CallTarget::Lowered(index) => state.functions().get_ptr_by_index(index),
        CallTarget::Import => None,
    }
}

/// Try to enter one lowered callee without returning to the transfer trampoline.
#[inline]
#[allow(clippy::too_many_arguments)]
fn try_step_direct_lowered_call(
    state: &mut StepState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    env: Option<Value>,
    copy_plan: Option<CopyRange>,
    resume_pc: usize,
) -> Option<Transfer> {
    // NOTE #Performance: keep this path specialized to avoid the transfer trampoline on hot direct calls
    let copy_plan = copy_plan?;

    // require one lowered target before entering the fast path
    let callee_ptr = resolve_direct_lowered_callee(state, target)?;
    let callee = unsafe { callee_ptr.as_ref() };

    // reject stack overflow before mutating any live state
    if state.engine.call_stack.len() >= state.options().limits.max_stack_depth {
        return Some(Transfer::Error(Error::StackOverflow));
    }

    // store the caller resume pc before allocating the callee
    {
        let caller = state.current_frame_mut();
        caller.resume_pc = resume_pc;
    }

    // allocate value and local storage for the callee
    let value_base = state.engine.value_stack.len();
    let local_base = state.engine.local_stack.len();
    state
        .engine
        .value_stack
        .resize(value_base + callee.value_count, Value::VOID);
    state
        .engine
        .local_stack
        .resize(local_base + callee.local_count, Value::VOID);

    let entry_block = &callee.blocks[callee.entry as usize];
    let entry_block_id = entry_block.mir_block;
    let entry_block_ptr = NonNull::from(entry_block);
    let mut new_frame = Frame::new(
        callee.frame_layout,
        function_id,
        callee_ptr,
        entry_block_ptr,
        entry_block_id,
        callee.entry as usize,
        value_base,
        callee.value_count,
        local_base,
        callee.local_count,
        env.unwrap_or(Value::VOID),
    );

    // bind parameters from the current caller frame
    let caller_index = state.frame_index;
    let current_function_ptr = {
        let frame = state.current_frame_mut();
        frame.function_ptr
    };
    let current_function = unsafe { current_function_ptr.as_ref() };
    let caller_ptr = {
        let Ok(caller) = state.frame_by_index(caller_index) else {
            return Some(Transfer::Error(Error::InvalidManagedReference));
        };
        caller as *const Frame
    };
    let caller = unsafe { &*caller_ptr };

    let new_frame_index = state.engine.call_stack.len();
    let heap_ptr = state.heap() as *const destack_heap::Heap;
    let frames_ptr = state.engine.call_stack.as_ptr();
    let frames_len = state.engine.call_stack.len();
    if let Err(error) = copy_values_with_plan_typed(
        state.executable,
        unsafe { &*heap_ptr },
        unsafe { std::slice::from_raw_parts(frames_ptr, frames_len) },
        &mut state.engine.value_stack,
        caller,
        &mut new_frame,
        new_frame_index,
        copy_plan,
        current_function.copy_pool.as_slice(),
    ) {
        return Some(Transfer::Error(error));
    }

    // push the callee frame and continue at its entry block
    state.engine.call_stack.push(new_frame);
    let new_index = state.engine.call_stack.len() - 1;
    state.enter_frame(new_index, callee);

    let entry_instructions = entry_block.instructions.as_slice();
    Some(step_instruction(state, entry_instructions, 0))
}

/// Enter a call with a resolved target function.
#[allow(clippy::too_many_arguments)]
fn call_with_target(
    state: &mut StepState<'_, '_>,
    dest: mir::Value,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Value>,
    copy_plan: Option<CopyRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> Transfer {
    // run the specialized lowered fast path when the caller allows it
    if allow_direct
        && let Some(transfer) =
            try_step_direct_lowered_call(state, function_id, target, env, copy_plan, resume_pc)
    {
        return transfer;
    }

    // otherwise bounce through the general transfer path
    Transfer::Call {
        function: function_id.id,
        target,
        destination: dest,
        arguments,
        env,
        copies: copy_plan,
        resume_pc,
    }
}

/// Enter a call terminator with explicit normal and unwind continuations.
#[allow(clippy::too_many_arguments)]
fn call_branch_with_target(
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Value>,
    normal_resume_point: engine::ResumePointId,
    unwind_resume_point: engine::ResumePointId,
) -> Transfer {
    // exceptional calls always go through the general transfer path
    Transfer::CallBranch {
        function: function_id.id,
        target,
        arguments,
        env,
        normal_resume_point,
        unwind_resume_point,
    }
}

/// Step function call (returns to trampoline).
pub(crate) fn step_call(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Call {
        dest,
        function,
        target,
        arguments,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve target function id
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let copy_plan = Some(*copies);

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats && state.options().limits.max_instructions.is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        *target,
        *arguments,
        None,
        copy_plan,
        pc + 1,
        allow_direct,
    )
}

/// Step exceptional direct call terminator.
pub(crate) fn step_call_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::CallBranch {
        function,
        target,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);

    call_branch_with_target(
        function_id,
        *target,
        *arguments,
        None,
        *normal_resume_point,
        *unwind_resume_point,
    )
}

/// Step virtual call (returns to trampoline).
pub(crate) fn step_call_virtual(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CallVirtual {
        dest,
        receiver,
        managed_pointee,
        slot_id,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *managed_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats && state.options().limits.max_instructions.is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        target,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Step exceptional virtual call terminator.
pub(crate) fn step_call_virtual_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::CallVirtualBranch {
        receiver,
        managed_pointee,
        slot_id,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *managed_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    call_branch_with_target(
        function_id,
        target,
        *arguments,
        None,
        *normal_resume_point,
        *unwind_resume_point,
    )
}

/// Step interface call (returns to trampoline).
pub(crate) fn step_call_interface(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CallInterface {
        dest,
        receiver,
        managed_pointee,
        slot_id,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id = match resolve_interface_dispatch_target(
        state,
        receiver_value,
        *managed_pointee,
        *slot_id,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats && state.options().limits.max_instructions.is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        target,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Step exceptional interface call terminator.
pub(crate) fn step_call_interface_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::CallInterfaceBranch {
        receiver,
        managed_pointee,
        slot_id,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id = match resolve_interface_dispatch_target(
        state,
        receiver_value,
        *managed_pointee,
        *slot_id,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    call_branch_with_target(
        function_id,
        target,
        *arguments,
        None,
        *normal_resume_point,
        *unwind_resume_point,
    )
}

/// Step indirect call (returns to trampoline).
pub(crate) fn step_call_indirect(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CallIndirect {
        dest,
        callee,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // resolve callable code and environment
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    // resolve the semantic call target directly
    let resolved_target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // return control to trampoline
    Transfer::Call {
        function,
        target: resolved_target,
        destination: *dest,
        arguments: *arguments,
        env,
        copies: None,
        resume_pc: pc + 1,
    }
}

/// Step exceptional indirect call terminator.
pub(crate) fn step_call_indirect_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::CallIndirectBranch {
        callee,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let callee_val = state.get(*callee);
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let resolved_target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    call_branch_with_target(
        function_id,
        resolved_target,
        *arguments,
        env,
        *normal_resume_point,
        *unwind_resume_point,
    )
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    state: &mut StepState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    callee: &Function,
    argument_values: &[super::bind::TransferredValue],
    env: Option<Value>,
) -> Result<(), Error> {
    // resolve frame bounds
    let (value_base, local_base) = {
        let frame = state.current_frame_mut();
        (frame.value_base, frame.local_base)
    };

    // clear frame local stack allocations
    state.current_frame_mut().stack_allocations.clear();

    // resize stacks to callee requirements
    let value_end = value_base + callee.value_count;
    let local_end = local_base + callee.local_count;
    resize_and_clear_stack(&mut state.engine.value_stack, value_base, value_end);
    resize_and_clear_stack(&mut state.engine.local_stack, local_base, local_end);

    // update frame metadata
    let entry_block = &callee.blocks[callee.entry as usize];
    {
        let frame = state.current_frame_mut();
        frame.frame_layout = callee.frame_layout;
        frame.function = function_id;
        frame.function_ptr = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.block_index = callee.entry as usize;
        frame.resume_pc = 0;
        frame.value_count = callee.value_count;
        frame.local_count = callee.local_count;
        frame.environment = env.unwrap_or(Value::VOID);
    }

    // refresh cached pointers for the new function
    state.refresh_for_function(callee);

    // bind function parameters
    let executable = state.executable;
    let frame_index = state.frame_index;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    bind_parameters_from_transferred_values(
        executable,
        &mut state.engine.value_stack,
        frame,
        frame_index,
        callee.argument_pool.as_slice(),
        callee.parameters,
        argument_values,
    )?;

    // update statistics
    if state.collect_stats {
        state.engine.statistics.calls_made += 1;
    }

    // keep frame ready for entry execution
    Ok(())
}

/// Step tail call to function.
pub(crate) fn step_tail_call(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::TailCall {
        function,
        target,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve the lowered fast path target
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let resolved_index = match *target {
        CallTarget::Lowered(index) => Some(index),
        CallTarget::Import => None,
    };

    // fall back to trampoline for unresolved targets
    let Some(resolved_index) = resolved_index else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            copies: Some(*copies),
        };
    };
    let Some(callee_ptr) = state.functions().get_ptr_by_index(resolved_index) else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            copies: Some(*copies),
        };
    };
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let argument_values = {
        let function_ptr = state.current_frame_mut().function_ptr;
        let current_func = unsafe { function_ptr.as_ref() };
        let caller = match state.frame_by_index(state.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match collect_transferred_values_from_copies(
            state.executable,
            state.heap(),
            state.engine.call_stack.as_slice(),
            &state.engine.value_stack,
            caller,
            current_func.copy_pool.as_slice(),
            *copies,
        ) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };

    // enter tail call fast path
    if let Err(error) = enter_tail_call(state, function_id, callee, &argument_values, None) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become step_instruction(state, entry_instructions, 0)
}

/// Step self tail call by reusing the current frame.
pub(crate) fn step_tail_call_self(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::TailCallSelf { entry, arguments } = &block[pc].data else {
        unreachable!()
    };

    // resolve current function entry block
    let (function_ptr, value_base, value_count, local_base, local_count) = {
        let frame = state.current_frame_mut();
        (
            frame.function_ptr,
            frame.value_base,
            frame.value_count,
            frame.local_base,
            frame.local_count,
        )
    };
    let function = unsafe { function_ptr.as_ref() };
    let entry_block = &function.blocks[*entry as usize];

    // collect argument values before clearing the frame
    let args = {
        let caller = match state.frame_by_index(state.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match collect_transferred_values_range(
            state.executable,
            state.heap(),
            state.engine.call_stack.as_slice(),
            &state.engine.value_stack,
            caller,
            function.argument_pool.as_slice(),
            *arguments,
        ) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };

    if state.collect_stats {
        state.engine.statistics.calls_made += 1;
    }

    // clear frame-local stack allocations
    state.current_frame_mut().stack_allocations.clear();

    // clear value and local slots
    let value_end = value_base + value_count;
    state.engine.value_stack[value_base..value_end].fill(Value::VOID);
    let local_end = local_base + local_count;
    state.engine.local_stack[local_base..local_end].fill(Value::VOID);

    // update frame to entry block
    {
        let frame = state.current_frame_mut();
        frame.block_index = *entry as usize;
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.resume_pc = 0;
    }

    // bind function parameters
    let executable = state.executable;
    let frame_index = state.frame_index;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    if let Err(error) = bind_parameters_from_transferred_values(
        executable,
        &mut state.engine.value_stack,
        frame,
        frame_index,
        function.argument_pool.as_slice(),
        function.parameters,
        &args,
    ) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_instructions = entry_block.instructions.as_slice();
    become step_instruction(state, entry_instructions, 0)
}

/// Step virtual tail call (returns to trampoline).
pub(crate) fn step_tail_call_virtual(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::TailCallVirtual {
        receiver,
        managed_pointee,
        slot_id,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *managed_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    Transfer::TailCall {
        function: function_id.id,
        target,
        arguments: *arguments,
        env: None,
        copies: None,
    }
}

/// Step interface tail call (returns to trampoline).
pub(crate) fn step_tail_call_interface(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::TailCallInterface {
        receiver,
        managed_pointee,
        slot_id,
        arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id = match resolve_interface_dispatch_target(
        state,
        receiver_value,
        *managed_pointee,
        *slot_id,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    Transfer::TailCall {
        function: function_id.id,
        target,
        arguments: *arguments,
        env: None,
        copies: None,
    }
}

/// Step indirect tail call.
pub(crate) fn step_tail_call_indirect(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::TailCallIndirect { callee, arguments } = &block[pc].data else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // resolve callable code and environment
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    // resolve the semantic call target
    let target = match state.functions().resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // fall back to trampoline for imported targets
    let resolved_index = match target {
        CallTarget::Lowered(index) => Some(index),
        CallTarget::Import => None,
    };
    let Some(resolved_index) = resolved_index else {
        return Transfer::TailCall {
            function,
            target,
            arguments: *arguments,
            env,
            copies: None,
        };
    };
    let Some(callee_ptr) = state.functions().get_ptr_by_index(resolved_index) else {
        return Transfer::TailCall {
            function,
            target,
            arguments: *arguments,
            env,
            copies: None,
        };
    };
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let caller = match state.frame_by_index(state.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = unsafe { caller.function_ptr.as_ref() };
    let argument_values = match collect_transferred_values_range(
        state.executable,
        state.heap(),
        state.engine.call_stack.as_slice(),
        &state.engine.value_stack,
        caller,
        caller_function.argument_pool.as_slice(),
        *arguments,
    ) {
        Ok(arguments) => arguments,
        Err(error) => return Transfer::Error(error),
    };

    // enter tail call fast path
    if let Err(error) = enter_tail_call(state, function_id, callee, &argument_values, env) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become step_instruction(state, entry_instructions, 0)
}
