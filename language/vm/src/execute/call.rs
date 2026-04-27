use destack_engine as engine;

use super::frame::{
    FrameValue, move_values, read_arguments, read_planned_arguments, write_parameters,
};
use super::prelude::*;

/// Load one lowered dispatch table field from a receiver.
fn load_receiver_field(
    state: &mut DispatchState<'_, '_>,
    receiver: Word,
    field: Option<FieldAccess>,
) -> Result<Word, Error> {
    let Some(field) = field else {
        return Err(Error::ConcreteMirRequired {
            context: "dispatch table field".to_string(),
        });
    };

    match field.pointer_class {
        PointerClass::Heap => {
            access::load_field_heap(state, receiver.as_heap_reference(), field, 0, None)
        }
        PointerClass::SharedHeap => access::load_field_shared_heap(
            state,
            receiver.as_shared_heap_reference(),
            field,
            0,
            None,
        ),
        _ => Err(Error::TypeMismatch {
            expected: "heap receiver".to_string(),
            actual: format!("{receiver:?}"),
        }),
    }
}

/// Resolve the vtable dispatch target for a virtual call.
fn resolve_virtual_dispatch_target(
    state: &mut DispatchState<'_, '_>,
    receiver: Word,
    table_field: Option<FieldAccess>,
    method_index: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field(state, receiver, table_field)?;
    let vtable_pointer = vtable_value.as_static_pointer();

    // load the function pointer from static table data
    let byte_offset = (method_index as usize)
        .checked_mul(Word::BYTE_LEN)
        .ok_or(Error::InvalidInstruction)?;
    let entry_address = vtable_pointer
        .address()
        .checked_add(byte_offset)
        .ok_or(Error::InvalidInstruction)?;
    let function = unsafe { *(entry_address as *const Word) };
    let function = function.as_function_pointer();
    let function = mir::LocalNodeId::new(function.function_index());

    Ok(function)
}

/// Resolve the itab dispatch target for an interface call.
fn resolve_interface_dispatch_target(
    state: &mut DispatchState<'_, '_>,
    receiver: Word,
    table_field: Option<FieldAccess>,
    method_index: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the itab id from the interface reference
    let itab_value = load_receiver_field(state, receiver, table_field)?;

    // decode the itab id
    let raw_id = itab_value.as_u64();
    let raw_id = u32::try_from(raw_id).map_err(|_| Error::InvalidInstruction)?;
    let table_id = mir::ItabId::new(raw_id);

    // resolve the itab entry for the interface call
    let table = state.tree().metadata.dispatch.itab(table_id);
    let entry = table
        .entries
        .get(method_index as usize)
        .ok_or(Error::InvalidInstruction)?;

    // require an interface method
    let mir::ItabEntry::Method { target_method, .. } = entry else {
        return Err(Error::InvalidInstruction);
    };

    Ok(*target_method)
}

/// Resolve one indirect callable into function code and environment.
fn resolve_indirect_callable(
    state: &mut DispatchState<'_, '_>,
    callable: Word,
    callable_type: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Function>, Option<Word>), Error> {
    // dispatch by the actual callee SSA type, not the code signature
    let callable_type = crate::program::repr_type(state.tree(), callable_type);
    match state.tree().get(callable_type) {
        mir::Type::FunctionPointer { .. } => {
            let function = mir::LocalNodeId::new(callable.as_function_pointer().function_index());

            Ok((function, None))
        }
        mir::Type::Callable { .. } => {
            let (function, environment_value) = access::decode_callable(state, callable)?;
            let function = mir::LocalNodeId::new(function.as_function_pointer().function_index());

            Ok((function, Some(environment_value)))
        }
        _ => Err(Error::InvalidInstruction),
    }
}

/// Resolve the expected signature for one indirect callable type.
fn indirect_callable_signature(
    tree: &mir::Tree,
    callable_type: mir::LocalNodeId<mir::Type>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let callable_type = crate::program::repr_type(tree, callable_type);
    match tree.get(callable_type) {
        mir::Type::FunctionPointer { signature } | mir::Type::Callable { signature } => {
            signature.ty().ok_or(Error::InvalidInstruction)
        }
        _ => Err(Error::InvalidInstruction),
    }
}

/// Check whether one function has the expected call signature.
fn validate_indirect_signature(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
    signature: mir::LocalNodeId<mir::Type>,
) -> Result<(), Error> {
    let mir::Type::FunctionSignature { parameters, result } = tree.get(signature) else {
        return Err(Error::InvalidInstruction);
    };
    let function = tree.get(function_id);

    if parameters.len() != function.parameters.len() {
        return Err(Error::TypeMismatch {
            expected: format!("function signature {signature:?}"),
            actual: format!("function {function_id:?}"),
        });
    }

    for (parameter, expected) in function.parameters.iter().zip(parameters) {
        if parameter.ty != *expected {
            return Err(Error::TypeMismatch {
                expected: format!("function signature {signature:?}"),
                actual: format!("function {function_id:?}"),
            });
        }
    }

    if function.return_type != (*result).into() {
        return Err(Error::TypeMismatch {
            expected: format!("function signature {signature:?}"),
            actual: format!("function {function_id:?}"),
        });
    }

    Ok(())
}

/// Load a function pointer.
pub(crate) fn execute_function_addr(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::FunctionAddr { dest, function } = &block[pc].operands else {
        unreachable!()
    };

    // build function pointer value
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let value = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));

    // store result
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Build a callable value from one function and environment.
pub(crate) fn execute_callable_bind(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::CallableBind {
        dest,
        function,
        environment,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // allocate the erased callable payload
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
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
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Load the callable environment pointer for the current frame.
pub(crate) fn execute_callable_environment(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CallableEnvironment { dest } = &block[pc].operands else {
        unreachable!()
    };

    // load current frame environment
    let frame_layout = state.frame_layout() as *const engine::FrameLayout;
    let environment = match state
        .current_frame_mut()
        .environment(unsafe { &*frame_layout })
    {
        Ok(environment) => environment,
        Err(error) => return Transfer::Error(error),
    };
    if environment == Word::VOID {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // store result
    state.set_word(*dest, environment);

    // continue to next instruction
    Transfer::Continue
}

/// Resolve one lowered direct-call target for the dispatch fast path.
#[inline]
fn resolve_direct_local_callee(
    state: &DispatchState<'_, '_>,
    target: CallTarget,
) -> Option<NonNull<crate::program::Function>> {
    // only lowered targets can use the direct fast path
    match target {
        CallTarget::Local(index) => state.program.functions.get_ptr_by_index(index),
        CallTarget::Import => None,
    }
}

/// Try to enter one lowered callee without returning to the transfer trampoline.
#[inline]
#[allow(clippy::too_many_arguments)]
fn try_execute_direct_local_call(
    state: &mut DispatchState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    env: Option<Word>,
    move_plan: Option<MoveRange>,
    resume_pc: usize,
) -> Option<Transfer> {
    // NOTE #Performance: keep this path specialized to avoid the transfer trampoline on hot direct calls
    let move_plan = move_plan?;

    // require one lowered target before entering the fast path
    let callee_ptr = resolve_direct_local_callee(state, target)?;
    let callee = unsafe { callee_ptr.as_ref() };

    // reject stack overflow before mutating any live state
    if state.engine.frames.len() >= state.options().limits.max_stack_depth {
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
    let layout = match state.program.frame_layout_by_id(callee.frame_layout) {
        Some(layout) => layout,
        None => return Some(Transfer::Error(Error::InvalidInstruction)),
    };
    let (stack_offset, frame_base) = match state.engine.allocate_frame(layout, state.options) {
        Ok(frame) => frame,
        Err(error) => return Some(Transfer::Error(error.error)),
    };
    let mut new_frame = Frame::new(
        callee.frame_layout,
        function_id,
        callee_ptr,
        entry_block_ptr,
        entry_block_id,
        layout,
        stack_offset,
        frame_base,
    );
    new_frame.set_environment(layout, env.unwrap_or(Word::VOID));

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

    if let Err(error) = move_values(
        state.program,
        caller,
        &mut new_frame,
        move_plan,
        current_function.move_pool.as_slice(),
    ) {
        return Some(Transfer::Error(error));
    }

    // push the callee frame and continue at its entry block
    state.engine.frames.push(new_frame);
    let new_index = state.engine.frames.len() - 1;
    state.enter_frame(new_index, callee);

    let entry_instructions = entry_block.instructions.as_slice();
    Some(dispatch_instruction(state, entry_instructions, 0))
}

/// Enter a call with a resolved target function.
#[allow(clippy::too_many_arguments)]
fn call_with_target(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
    move_plan: Option<MoveRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> Transfer {
    // run the specialized lowered fast path when the caller allows it
    if allow_direct
        && let Some(transfer) =
            try_execute_direct_local_call(state, function_id, target, env, move_plan, resume_pc)
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
        moves: move_plan,
        resume_pc,
    }
}

/// Enter a call terminator with explicit normal and unwind continuations.
#[allow(clippy::too_many_arguments)]
fn call_branch_with_target(
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
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

/// Execute function call (returns to trampoline).
pub(crate) fn execute_call(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Call {
        dest,
        function,
        target,
        arguments,
        moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve target function id
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let move_plan = Some(*moves);

    // skip fast path when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

    call_with_target(
        state,
        *dest,
        function_id,
        *target,
        *arguments,
        None,
        move_plan,
        pc + 1,
        allow_direct,
    )
}

/// Execute exceptional direct call terminator.
pub(crate) fn execute_call_branch(
    _state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::CallBranch {
        function,
        target,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].operands
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

/// Execute virtual call (returns to trampoline).
pub(crate) fn execute_call_virtual(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CallVirtual {
        dest,
        receiver,
        table_field,
        method_index,
        arguments,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // skip fast path when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

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

/// Execute exceptional virtual call terminator.
pub(crate) fn execute_call_virtual_branch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::CallVirtualBranch {
        receiver,
        table_field,
        method_index,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
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

/// Execute interface call (returns to trampoline).
pub(crate) fn execute_call_interface(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CallInterface {
        dest,
        receiver,
        table_field,
        method_index,
        arguments,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *table_field, *method_index)
        {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
        Some(target) => target,
        None => {
            return Transfer::Error(Error::UndefinedFunction {
                function: function_id,
            });
        }
    };

    // skip fast path when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

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

/// Execute exceptional interface call terminator.
pub(crate) fn execute_call_interface_branch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::CallInterfaceBranch {
        receiver,
        table_field,
        method_index,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *table_field, *method_index)
        {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
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

/// Execute indirect call (returns to trampoline).
pub(crate) fn execute_call_indirect(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CallIndirect {
        dest,
        callee,
        arguments,
    } = &block[pc].operands
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
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }

    // resolve the semantic call target directly
    let resolved_target = match state.program.functions.resolve(function_id) {
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
        moves: None,
        resume_pc: pc + 1,
    }
}

/// Execute exceptional indirect call terminator.
pub(crate) fn execute_call_indirect_branch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::CallIndirectBranch {
        callee,
        arguments,
        normal_resume_point,
        unwind_resume_point,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let callee_val = state.get(*callee);
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }
    let resolved_target = match state.program.functions.resolve(function_id) {
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
    state: &mut DispatchState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    callee: &Function,
    argument_values: &[FrameValue],
    env: Option<Word>,
) -> Result<(), Error> {
    // update frame metadata
    let entry_block = &callee.blocks[callee.entry as usize];
    let layout = state
        .program
        .frame_layout_by_id(callee.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let (stack_offset, frame_base) = state
        .engine
        .allocate_frame(layout, state.options)
        .map_err(|error| error.error)?;
    {
        let frame = state.current_frame_mut();
        frame.frame_layout = callee.frame_layout;
        frame.function = function_id;
        frame.function_ptr = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.current_block = entry_block.mir_block;
        frame.resume_pc = 0;
        frame.replace_bytes(stack_offset, layout.byte_len as usize, frame_base);
        frame.set_environment(layout, env.unwrap_or(Word::VOID));
    }

    // refresh cached pointers for the new function
    state.refresh_for_function(callee);

    // bind function parameters
    let program = state.program;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    write_parameters(
        program,
        frame,
        callee.argument_pool.as_slice(),
        callee.parameters,
        argument_values,
    )?;

    Ok(())
}

/// Execute tail call to function.
pub(crate) fn execute_tail_call(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TailCall {
        function,
        target,
        moves,
    } = &block[pc].operands
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
            moves: Some(*moves),
        };
    };
    let Some(callee_ptr) = state.program.functions.get_ptr_by_index(resolved_index) else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
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

        match read_planned_arguments(
            state.program,
            state.engine.frames.as_slice(),
            caller,
            current_func.move_pool.as_slice(),
            *moves,
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
    dispatch_instruction(state, entry_instructions, 0)
}

/// Execute self tail call by reusing the current frame.
pub(crate) fn execute_tail_call_self(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TailCallSelf { entry, arguments } = &block[pc].operands else {
        unreachable!()
    };

    // resolve current function entry block
    let function_ptr = {
        let frame = state.current_frame_mut();
        frame.function_ptr
    };
    let function = unsafe { function_ptr.as_ref() };
    let entry_block = &function.blocks[*entry as usize];

    // collect argument values before clearing the frame
    let args = {
        let caller = match state.frame_by_index(state.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match read_arguments(
            state.program,
            state.engine.frames.as_slice(),
            caller,
            function.argument_pool.as_slice(),
            *arguments,
        ) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };
    let frame_layout = state.frame_layout() as *const engine::FrameLayout;
    state
        .current_frame_mut()
        .clear_values(unsafe { &*frame_layout });

    // update frame to entry block
    {
        let frame = state.current_frame_mut();
        frame.block_ptr = NonNull::from(entry_block);
        frame.current_block = entry_block.mir_block;
        frame.resume_pc = 0;
    }

    // refresh cached frame pointers after replacing frame bytes
    state.refresh_for_function(function);

    // bind function parameters
    let program = state.program;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    if let Err(error) = write_parameters(
        program,
        frame,
        function.argument_pool.as_slice(),
        function.parameters,
        &args,
    ) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_instructions = entry_block.instructions.as_slice();
    dispatch_instruction(state, entry_instructions, 0)
}

/// Execute virtual tail call (returns to trampoline).
pub(crate) fn execute_tail_call_virtual(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TailCallVirtual {
        receiver,
        table_field,
        method_index,
        arguments,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_virtual_dispatch_target(state, receiver_value, *table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
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
        moves: None,
    }
}

/// Execute interface tail call (returns to trampoline).
pub(crate) fn execute_tail_call_interface(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TailCallInterface {
        receiver,
        table_field,
        method_index,
        arguments,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve dynamic target
    let receiver_value = state.get(*receiver);
    let function_id =
        match resolve_interface_dispatch_target(state, receiver_value, *table_field, *method_index)
        {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match state.program.functions.resolve(function_id) {
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
        moves: None,
    }
}

/// Execute indirect tail call.
pub(crate) fn execute_tail_call_indirect(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TailCallIndirect { callee, arguments } = &block[pc].operands else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // resolve callable code and environment
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callable(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }

    // resolve the semantic call target
    let target = match state.program.functions.resolve(function_id) {
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
            moves: None,
        };
    };
    let Some(callee_ptr) = state.program.functions.get_ptr_by_index(resolved_index) else {
        return Transfer::TailCall {
            function,
            target,
            arguments: *arguments,
            env,
            moves: None,
        };
    };
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let caller = match state.frame_by_index(state.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = unsafe { caller.function_ptr.as_ref() };
    let argument_values = match read_arguments(
        state.program,
        state.engine.frames.as_slice(),
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
    dispatch_instruction(state, entry_instructions, 0)
}
