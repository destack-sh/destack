use super::*;

/// Load a field value from a heap aggregate receiver.
fn load_receiver_field(
    state: &mut ThreadedState<'_, '_>,
    receiver: Value,
    managed_pointee: Option<mir::LocalNodeId<mir::Type>>,
    field_index: u32,
) -> Result<Value, Error> {
    // resolve based on receiver storage
    match receiver.tag() {
        ValueTag::ManagedReference => {
            let Some(managed_pointee) = managed_pointee else {
                return Err(Error::InvalidManagedReference);
            };

            let handle = receiver.as_managed_reference().unwrap();
            instruction::load_field_managed(
                state,
                handle,
                managed_pointee,
                field_index,
                UNKNOWN_FIELD_COUNT,
            )
        }
        ValueTag::Aggregate | ValueTag::String => {
            instruction::get_field(state, receiver, field_index)
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{receiver:?}"),
        }),
    }
}

/// Resolve the vtable dispatch target for a virtual call.
fn resolve_virtual_dispatch_target(
    state: &mut ThreadedState<'_, '_>,
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
        .interpreter
        .isolate
        .vtable_for_global(vtable_pointer.id)
        .ok_or(Error::InvalidInstruction)?;

    // resolve the vtable slot for the virtual call
    let table = state
        .interpreter
        .isolate
        .image
        .tree
        .dispatch_table
        .vtable(table_id);
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
    state: &mut ThreadedState<'_, '_>,
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
    let table = state
        .interpreter
        .isolate
        .image
        .tree
        .dispatch_table
        .itab(table_id);
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

/// Load a function pointer.
pub(crate) fn handle_function_addr(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FunctionAddr { dest, function } = &block[pc].data else {
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

/// Load the closure environment pointer for the current frame.
pub(crate) fn handle_function_env(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FunctionEnv { dest } = &block[pc].data else {
        unreachable!()
    };

    // load current frame closure env
    let env = state.current_frame_mut().closure_env;
    if env == Value::VOID {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    // store result
    state.set(*dest, env);

    // continue to next instruction
    next!(state, block, pc)
}

/// Enter a call with a resolved target function.
#[allow(clippy::too_many_arguments)]
fn call_with_target(
    state: &mut ThreadedState<'_, '_>,
    dest: mir::Value,
    function_id: mir::LocalNodeId<mir::Function>,
    callee_index: u32,
    arguments: ArgumentRange,
    env: Option<Value>,
    copy_plan: Option<CopyRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> ControlFlow {
    // try direct threaded call when possible
    if allow_direct
        && copy_plan.is_some()
        && !state
            .interpreter
            .engine
            .threaded_functions
            .is_import(function_id)
    {
        let resolved_index = if callee_index == INVALID_FUNCTION_INDEX {
            state
                .interpreter
                .engine
                .threaded_functions
                .index_for(function_id)
        } else {
            Some(callee_index)
        };
        let callee_ptr = resolved_index.and_then(|index| {
            state
                .interpreter
                .engine
                .threaded_functions
                .get_ptr_by_index(index)
        });
        if let Some(callee_ptr) = callee_ptr {
            let callee = unsafe { callee_ptr.as_ref() };

            // check stack overflow
            if state.interpreter.engine.call_stack.len()
                >= state
                    .interpreter
                    .isolate
                    .image
                    .options
                    .limits
                    .max_stack_depth
            {
                return ControlFlow::Error(Error::StackOverflow);
            }

            // store return destination and resume pc on caller frame
            {
                let caller = state.current_frame_mut();
                caller.return_destination = dest;
                caller.resume_pc = resume_pc;
            }

            // allocate new frame for callee
            let value_base = state.interpreter.engine.value_stack.len();
            let local_base = state.interpreter.engine.local_stack.len();
            state
                .interpreter
                .engine
                .value_stack
                .resize(value_base + callee.value_count, Value::VOID);
            state
                .interpreter
                .engine
                .local_stack
                .resize(local_base + callee.local_count, Value::VOID);
            let entry_block = &callee.blocks[callee.entry as usize];
            let entry_block_id = entry_block.mir_block;
            let entry_block_ptr = NonNull::from(entry_block);
            let new_frame = Frame::new(
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

            // bind parameters from caller values
            let caller_index = state.frame_index;
            let current_threaded_ptr = {
                let frame = state.current_frame_mut();
                frame.threaded
            };
            let current_func = unsafe { current_threaded_ptr.as_ref() };
            let caller_ptr = {
                let Ok(caller) = state.frame_by_index(caller_index) else {
                    return ControlFlow::Error(Error::InvalidManagedReference);
                };
                caller as *const Frame
            };
            let caller = unsafe { &*caller_ptr };
            let Some(copy_plan) = copy_plan else {
                return ControlFlow::Error(Error::InvalidInstruction);
            };
            copy_values_with_plan(
                &mut state.interpreter.engine.value_stack,
                caller,
                &new_frame,
                copy_plan,
                current_func.copy_pool.as_slice(),
            );

            // push new frame and refresh state
            state.interpreter.engine.call_stack.push(new_frame);
            let new_index = state.interpreter.engine.call_stack.len() - 1;
            state.enter_frame(new_index, callee);

            // continue at entry block
            let entry_instructions = entry_block.instructions.as_slice();
            return (entry_instructions[0].handler)(state, entry_instructions, 0);
        }
    }

    // return control to trampoline
    ControlFlow::Call {
        function: function_id.id,
        callee_index,
        destination: dest,
        arguments,
        env,
        copies: copy_plan,
        resume_pc,
    }
}

/// Handle function call (returns to trampoline).
pub(crate) fn handle_call(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Call {
        dest,
        function,
        callee_index,
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
    let allow_direct = !state.collect_stats
        && state
            .interpreter
            .isolate
            .image
            .options
            .limits
            .max_instructions
            .is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        *callee_index,
        *arguments,
        None,
        copy_plan,
        pc + 1,
        allow_direct,
    )
}

/// Handle virtual call (returns to trampoline).
pub(crate) fn handle_call_virtual(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CallVirtual {
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
            Err(error) => return ControlFlow::Error(error),
        };

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats
        && state
            .interpreter
            .isolate
            .image
            .options
            .limits
            .max_instructions
            .is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        INVALID_FUNCTION_INDEX,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Handle interface call (returns to trampoline).
pub(crate) fn handle_call_interface(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CallInterface {
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
        Err(error) => return ControlFlow::Error(error),
    };

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats
        && state
            .interpreter
            .isolate
            .image
            .options
            .limits
            .max_instructions
            .is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        INVALID_FUNCTION_INDEX,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Handle indirect call (returns to trampoline).
pub(crate) fn handle_call_indirect(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CallIndirect {
        dest,
        callee,
        env,
        arguments,
        cached_function,
        cached_index,
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

    // reuse cached callee index when possible
    if cached_function.get() == Some(function) {
        let cached_index = cached_index.get().unwrap_or(INVALID_FUNCTION_INDEX);
        return ControlFlow::Call {
            function,
            callee_index: cached_index,
            destination: *dest,
            arguments: *arguments,
            env: env.map(|value| state.get(value)),
            copies: None,
            resume_pc: pc + 1,
        };
    }

    // resolve callee index and update cache
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let resolved_index = state
        .interpreter
        .engine
        .threaded_functions
        .index_for(function_id)
        .unwrap_or(INVALID_FUNCTION_INDEX);
    cached_function.set(Some(function));
    cached_index.set(Some(resolved_index));

    // return control to trampoline
    ControlFlow::Call {
        function,
        callee_index: resolved_index,
        destination: *dest,
        arguments: *arguments,
        env: env.map(|value| state.get(value)),
        copies: None,
        resume_pc: pc + 1,
    }
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    state: &mut ThreadedState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    callee: &ThreadedFunction,
    argument_values: &[Value],
    env: Option<Value>,
) {
    // resolve frame bounds
    let (value_base, local_base) = {
        let frame = state.current_frame_mut();
        (frame.value_base, frame.local_base)
    };

    // clear frame local stack allocations
    state.current_frame_mut().stack_values.clear();

    // resize stacks to callee requirements
    let value_end = value_base + callee.value_count;
    let local_end = local_base + callee.local_count;
    resize_and_clear_stack(
        &mut state.interpreter.engine.value_stack,
        value_base,
        value_end,
    );
    resize_and_clear_stack(
        &mut state.interpreter.engine.local_stack,
        local_base,
        local_end,
    );

    // update frame metadata
    let entry_block = &callee.blocks[callee.entry as usize];
    {
        let frame = state.current_frame_mut();
        frame.function = function_id;
        frame.threaded = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.block_index = callee.entry as usize;
        frame.resume_pc = 0;
        frame.value_count = callee.value_count;
        frame.local_count = callee.local_count;
        frame.closure_env = env.unwrap_or(Value::VOID);
    }

    // refresh cached pointers for the new threaded function
    state.refresh_for_threaded(callee);

    // bind function parameters
    let parameter_slice = callee.parameters.slice(callee.argument_pool.as_slice());
    // use direct indexing when arguments cover parameters
    if argument_values.len() >= parameter_slice.len() {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = argument_values[index];
            state.set(*param, value);
        }
    }
    // fall back to defaulted arguments
    else {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = argument_values.get(index).copied().unwrap_or(Value::VOID);
            state.set(*param, value);
        }
    }

    // update statistics
    if state.collect_stats {
        state.interpreter.engine.statistics.calls_made += 1;
    }

    // keep frame ready for entry execution
}

/// Handle tail call to function.
pub(crate) fn handle_tail_call(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCall {
        function,
        callee_index,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve callee index
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let resolved_index = if *callee_index == INVALID_FUNCTION_INDEX {
        state
            .interpreter
            .engine
            .threaded_functions
            .index_for(function_id)
    } else {
        Some(*callee_index)
    };

    // fall back to trampoline for non-threaded targets
    let Some(resolved_index) = resolved_index else {
        return ControlFlow::TailCall {
            function: *function,
            callee_index: *callee_index,
            arguments: ArgumentRange::empty(),
            env: None,
            copies: Some(*copies),
        };
    };
    let Some(callee_ptr) = state
        .interpreter
        .engine
        .threaded_functions
        .get_ptr_by_index(resolved_index)
    else {
        return ControlFlow::TailCall {
            function: *function,
            callee_index: *callee_index,
            arguments: ArgumentRange::empty(),
            env: None,
            copies: Some(*copies),
        };
    };
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let argument_values = {
        let threaded_ptr = state.current_frame_mut().threaded;
        let current_func = unsafe { threaded_ptr.as_ref() };
        let copy_pairs = copies.slice(current_func.copy_pool.as_slice());
        let mut args: SmallVec<[Value; 16]> = SmallVec::with_capacity(copy_pairs.len());

        for pair in copy_pairs {
            let value = if pair.src == INVALID_VALUE_ID {
                Value::VOID
            } else {
                let arg_value = mir::Value::new(pair.src);
                state.get(arg_value)
            };
            args.push(value);
        }

        args
    };

    // enter tail call fast path
    enter_tail_call(state, function_id, callee, &argument_values, None);

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}

/// Handle self tail call by reusing the current frame.
pub(crate) fn handle_tail_call_self(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallSelf { entry, arguments } = &block[pc].data else {
        unreachable!()
    };

    // collect argument values
    let args = collect_values(state, *arguments);

    if state.collect_stats {
        state.interpreter.engine.statistics.calls_made += 1;
    }

    // resolve current function entry block
    let (threaded_ptr, value_base, value_count, local_base, local_count) = {
        let frame = state.current_frame_mut();
        (
            frame.threaded,
            frame.value_base,
            frame.value_count,
            frame.local_base,
            frame.local_count,
        )
    };
    let threaded = unsafe { threaded_ptr.as_ref() };
    let entry_block = &threaded.blocks[*entry as usize];

    // clear frame-local stack allocations
    state.current_frame_mut().stack_values.clear();

    // clear value and local slots
    let value_end = value_base + value_count;
    state.interpreter.engine.value_stack[value_base..value_end].fill(Value::VOID);
    let local_end = local_base + local_count;
    state.interpreter.engine.local_stack[local_base..local_end].fill(Value::VOID);

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
    let parameter_slice = threaded.parameters.slice(threaded.argument_pool.as_slice());
    // use direct indexing when arguments cover parameters
    if args.len() >= parameter_slice.len() {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = args[index];
            state.set(*param, value);
        }
    }
    // fall back to defaulted arguments
    else {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = args.get(index).copied().unwrap_or(Value::VOID);
            state.set(*param, value);
        }
    }

    // continue at entry block
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}

/// Handle virtual tail call (returns to trampoline).
pub(crate) fn handle_tail_call_virtual(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallVirtual {
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
            Err(error) => return ControlFlow::Error(error),
        };

    ControlFlow::TailCall {
        function: function_id.id,
        callee_index: INVALID_FUNCTION_INDEX,
        arguments: *arguments,
        env: None,
        copies: None,
    }
}

/// Handle interface tail call (returns to trampoline).
pub(crate) fn handle_tail_call_interface(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallInterface {
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
        Err(error) => return ControlFlow::Error(error),
    };

    ControlFlow::TailCall {
        function: function_id.id,
        callee_index: INVALID_FUNCTION_INDEX,
        arguments: *arguments,
        env: None,
        copies: None,
    }
}

/// Handle indirect tail call.
pub(crate) fn handle_tail_call_indirect(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallIndirect {
        callee,
        env,
        arguments,
        cached_function,
        cached_ptr,
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

    // resolve callee id
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);

    // reuse cached callee pointer when possible
    if cached_function.get() == Some(function) {
        if let Some(callee_ptr) = cached_ptr.get() {
            let callee = unsafe { callee_ptr.as_ref() };
            let argument_values = collect_values(state, *arguments);
            enter_tail_call(
                state,
                function_id,
                callee,
                &argument_values,
                env.map(|value| state.get(value)),
            );
            let entry_block_ptr = state.current_frame_mut().block_ptr;
            let entry_block = unsafe { entry_block_ptr.as_ref() };
            let entry_instructions = entry_block.instructions.as_slice();
            become (entry_instructions[0].handler)(state, entry_instructions, 0)
        }

        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            env: env.map(|value| state.get(value)),
            copies: None,
        };
    }

    // resolve callee index
    let resolved_index = state
        .interpreter
        .engine
        .threaded_functions
        .index_for(function_id);

    // fall back to trampoline for non-threaded targets
    let Some(resolved_index) = resolved_index else {
        cached_function.set(Some(function));
        cached_ptr.set(None);
        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            env: env.map(|value| state.get(value)),
            copies: None,
        };
    };
    let Some(callee_ptr) = state
        .interpreter
        .engine
        .threaded_functions
        .get_ptr_by_index(resolved_index)
    else {
        cached_function.set(Some(function));
        cached_ptr.set(None);
        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            env: env.map(|value| state.get(value)),
            copies: None,
        };
    };
    cached_function.set(Some(function));
    cached_ptr.set(Some(callee_ptr));
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let argument_values = collect_values(state, *arguments);

    // enter tail call fast path
    enter_tail_call(
        state,
        function_id,
        callee,
        &argument_values,
        env.map(|value| state.get(value)),
    );

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}
