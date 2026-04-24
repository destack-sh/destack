use destack_engine as engine;

use super::bind::{
    bind_parameters_from_transferred_values, collect_transferred_values_from_copies,
    collect_transferred_values_range, copy_values_with_plan_typed,
};
use super::prelude::*;

const VTABLE_FIELD_INDEX: u32 = 0;
const INTERFACE_ITAB_FIELD_INDEX: u32 = 1;

/// Resolve one receiver field access from one managed pointee type.
fn receiver_field_access(
    state: &StepState<'_, '_>,
    heap_pointee: mir::LocalNodeId<mir::Type>,
    field_index: u32,
) -> Result<crate::module::FieldAccess, Error> {
    // load the compiled receiver layout
    let layout = state.layout(heap_pointee)?;
    let field = layout.field(field_index);

    // convert the selected field into one access plan
    field.map_or_else(
        || {
            Err(Error::TypeMismatch {
                expected: "receiver composite field".to_string(),
                actual: format!("{heap_pointee:?}"),
            })
        },
        |field| {
            let is_scalar = state
                .layout(field.ty)
                .is_ok_and(|layout| layout.is_scalar());

            Ok(crate::module::FieldAccess {
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
    heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
    field_index: u32,
) -> Result<Value, Error> {
    // resolve based on receiver storage
    match receiver.tag() {
        ValueTag::HeapReference => {
            let Some(heap_pointee) = heap_pointee else {
                return Err(Error::ConcreteMirRequired {
                    context: "receiver field access".to_string(),
                });
            };

            // resolve the receiver field access and load directly
            let handle = receiver
                .as_heap_reference()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "composite".to_string(),
                    actual: format!("{receiver:?}"),
                })?;
            let field = receiver_field_access(state, heap_pointee, field_index)?;
            access::load_field_heap(state, handle, field, field_index, None)
        }
        ValueTag::SharedHeapReference => {
            let Some(heap_pointee) = heap_pointee else {
                return Err(Error::ConcreteMirRequired {
                    context: "receiver field access".to_string(),
                });
            };

            let handle =
                receiver
                    .as_shared_heap_reference()
                    .ok_or_else(|| Error::TypeMismatch {
                        expected: "composite".to_string(),
                        actual: format!("{receiver:?}"),
                    })?;
            let field = receiver_field_access(state, heap_pointee, field_index)?;
            access::load_field_shared_heap(state, handle, field, field_index, None)
        }
        ValueTag::StackPointer => {
            let Some(heap_pointee) = heap_pointee else {
                return Err(Error::ConcreteMirRequired {
                    context: "receiver field access".to_string(),
                });
            };

            let pointer = receiver
                .as_stack_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "composite".to_string(),
                    actual: format!("{receiver:?}"),
                })?;
            let field = receiver_field_access(state, heap_pointee, field_index)?;
            access::load_field_stack(state, pointer, field, field_index, None)
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
    heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
    slot_id: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field(state, receiver, heap_pointee, VTABLE_FIELD_INDEX)?;

    // require a global pointer for the vtable
    let vtable_pointer = vtable_value
        .as_global_pointer()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "global_pointer".to_string(),
            actual: format!("{vtable_value:?}"),
        })?;

    // map the vtable global to a vtable id
    let table_id = state
        .module
        .vtable_id_by_global
        .get(&vtable_pointer.id)
        .copied()
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
    heap_pointee: Option<mir::LocalNodeId<mir::Type>>,
    slot_id: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the itab id from the interface reference
    let itab_value =
        load_receiver_field(state, receiver, heap_pointee, INTERFACE_ITAB_FIELD_INDEX)?;

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
    let callable_type = crate::module::repr_type(state.tree(), callable_type);
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
        mir::Type::Callable { .. } => {
            let (function, environment_value) = access::decode_callable(state, callable)?;
            let function = function
                .as_function_pointer()
                .ok_or_else(|| Error::TypeMismatch {
                    expected: "function_pointer".to_string(),
                    actual: format!("{function:?}"),
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
    // decode instruction immediate
    let Immediate::FunctionAddr { dest, function } = &block[pc].immediate else {
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
pub(crate) fn step_callable_bind(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::CallableBind {
        dest,
        function,
        environment,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // allocate the erased callable payload
    let function_id = mir::LocalNodeId::new(*function);
    let function = Value::function_pointer(function_id);
    let environment_value = state.get(*environment);
    let ty = match state.value_type(*dest) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::allocate_callable(state, ty, function, environment_value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Load the callable environment pointer for the current frame.
pub(crate) fn step_callable_environment(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::CallableEnvironment { dest } = &block[pc].immediate else {
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
fn resolve_direct_local_callee(
    state: &StepState<'_, '_>,
    target: CallTarget,
) -> Option<NonNull<crate::module::Function>> {
    // only lowered targets can use the direct fast path
    match target {
        CallTarget::Local(index) => state.module.functions.get_ptr_by_index(index),
        CallTarget::Import => None,
    }
}

/// Try to enter one lowered callee without returning to the transfer trampoline.
#[inline]
#[allow(clippy::too_many_arguments)]
fn try_step_direct_local_call(
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
    let callee_ptr = resolve_direct_local_callee(state, target)?;
    let callee = unsafe { callee_ptr.as_ref() };

    // reject stack overflow before mutating any live state
    if state.engine.stack.len() >= state.options().limits.max_stack_depth {
        return Some(Transfer::Error(Error::StackOverflow));
    }

    // store the caller resume pc before allocating the callee
    {
        let caller = state.current_frame_mut();
        caller.resume_pc = resume_pc;
    }

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
        callee.value_count,
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
            return Some(Transfer::Error(Error::InvalidHeapReference));
        };
        caller as *const Frame
    };
    let caller = unsafe { &*caller_ptr };

    let new_frame_index = state.engine.stack.len();
    let heap_ptr = state.heap() as *const destack_heap::Heap;
    let frames_ptr = state.engine.stack.as_ptr();
    let frames_len = state.engine.stack.len();
    if let Err(error) = copy_values_with_plan_typed(
        state.module,
        unsafe { &*heap_ptr },
        unsafe { std::slice::from_raw_parts(frames_ptr, frames_len) },
        caller,
        &mut new_frame,
        new_frame_index,
        copy_plan,
        current_function.copy_pool.as_slice(),
    ) {
        return Some(Transfer::Error(error));
    }

    // push the callee frame and continue at its entry block
    state.engine.stack.push(new_frame);
    let new_index = state.engine.stack.len() - 1;
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
            try_step_direct_local_call(state, function_id, target, env, copy_plan, resume_pc)
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

    // decode instruction immediate
    let Immediate::Call {
        dest,
        function,
        target,
        arguments,
        copies,
    } = &block[pc].immediate
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

    let Immediate::CallBranch {
        function,
        target,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].immediate
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

    // decode instruction immediate
    let Immediate::CallVirtual {
        dest,
        receiver,
        heap_pointee,
        slot_id,
        arguments,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    let Immediate::CallVirtualBranch {
        receiver,
        heap_pointee,
        slot_id,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    // decode instruction immediate
    let Immediate::CallInterface {
        dest,
        receiver,
        heap_pointee,
        slot_id,
        arguments,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    let Immediate::CallInterfaceBranch {
        receiver,
        heap_pointee,
        slot_id,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    // decode instruction immediate
    let Immediate::CallIndirect {
        dest,
        callee,
        arguments,
    } = &block[pc].immediate
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
    let resolved_target = match state.module.functions.resolve(function_id) {
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

    let Immediate::CallIndirectBranch {
        callee,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].immediate
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
    let resolved_target = match state.module.functions.resolve(function_id) {
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
    // clear frame local stack allocations
    state.current_frame_mut().stack_allocations.clear();

    // update frame metadata
    let entry_block = &callee.blocks[callee.entry as usize];
    {
        let frame = state.current_frame_mut();
        frame.reset_slots(callee.value_count, callee.local_count);
        frame.frame_layout = callee.frame_layout;
        frame.function = function_id;
        frame.function_ptr = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.block_index = callee.entry as usize;
        frame.resume_pc = 0;
        frame.environment = env.unwrap_or(Value::VOID);
    }

    // refresh cached pointers for the new function
    state.refresh_for_function(callee);

    // bind function parameters
    let module = state.module;
    let frame_index = state.frame_index;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    bind_parameters_from_transferred_values(
        module,
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

    // decode instruction immediate
    let Immediate::TailCall {
        function,
        target,
        copies,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve the lowered fast path target
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let resolved_index = match *target {
        CallTarget::Local(index) => Some(index),
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
    let Some(callee_ptr) = state.module.functions.get_ptr_by_index(resolved_index) else {
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
            state.module,
            state.heap(),
            state.engine.stack.as_slice(),
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

    // decode instruction immediate
    let Immediate::TailCallSelf { entry, arguments } = &block[pc].immediate else {
        unreachable!()
    };

    // resolve current function entry block
    let (function_ptr, value_count, local_count) = {
        let frame = state.current_frame_mut();
        (frame.function_ptr, frame.value_count, frame.local_count)
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
            state.module,
            state.heap(),
            state.engine.stack.as_slice(),
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
    state
        .current_frame_mut()
        .reset_slots(value_count, local_count);

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
    let module = state.module;
    let frame_index = state.frame_index;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    if let Err(error) = bind_parameters_from_transferred_values(
        module,
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

    // decode instruction immediate
    let Immediate::TailCallVirtual {
        receiver,
        heap_pointee,
        slot_id,
        arguments,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    // decode instruction immediate
    let Immediate::TailCallInterface {
        receiver,
        heap_pointee,
        slot_id,
        arguments,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *heap_pointee, *slot_id) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.module.functions.resolve(function_id) {
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

    // decode instruction immediate
    let Immediate::TailCallIndirect { callee, arguments } = &block[pc].immediate else {
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
    let target = match state.module.functions.resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // fall back to trampoline for imported targets
    let resolved_index = match target {
        CallTarget::Local(index) => Some(index),
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
    let Some(callee_ptr) = state.module.functions.get_ptr_by_index(resolved_index) else {
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
        state.module,
        state.heap(),
        state.engine.stack.as_slice(),
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
