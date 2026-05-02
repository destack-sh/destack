use std::ptr::NonNull;

use destack_engine as engine;

use super::dispatch::dispatch_block;
use super::frame::{
    FrameValue, load_arguments, load_planned_arguments, move_values, store_parameters,
};
use super::{access, callable};
use crate::diagnostic::Error;
use crate::interpreter::{DispatchState, Frame};
use crate::program::{
    ArgumentRange, Call, CallBranch, CallIndirect, CallIndirectBranch, CallInterface,
    CallInterfaceBranch, CallTarget, CallVirtual, CallVirtualBranch, CallableBind,
    CallableEnvironment, FieldAccess, FieldAccessId, Function, FunctionAddr, Instruction,
    MoveRange, PointerClass, TailCall, TailCallIndirect, TailCallInterface, TailCallSelf,
    TailCallVirtual, Transfer, repr_type,
};
use crate::{FunctionPointer, Word};
use destack_mir as mir;

/// Load one lowered call table field from a receiver.
fn load_receiver_field(
    state: &mut DispatchState<'_, '_>,
    receiver: Word,
    field: Option<FieldAccess>,
) -> Result<Word, Error> {
    let Some(field) = field else {
        return Err(Error::MissingRepresentation {
            context: "call table field".to_string(),
        });
    };

    match field.pointer_class {
        PointerClass::Heap => {
            access::load_field_heap(state, receiver.as_heap_reference(), field, 0, 1)
        }
        PointerClass::SharedHeap => {
            access::load_field_shared_heap(state, receiver.as_shared_heap_reference(), field, 0, 1)
        }
        _ => Err(Error::TypeMismatch {
            expected: "heap receiver".to_string(),
            actual: format!("{receiver:?}"),
        }),
    }
}

/// Return one pooled call table field.
fn load_table_field(
    state: &DispatchState<'_, '_>,
    field: Option<FieldAccessId>,
) -> Option<FieldAccess> {
    field.map(|field| state.field_access(field))
}

/// Resolve the callee for one virtual call.
fn resolve_virtual_callee(
    state: &mut DispatchState<'_, '_>,
    receiver: Word,
    table_field: Option<FieldAccess>,
    method_index: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field(state, receiver, table_field)?;
    let vtable_pointer = vtable_value.as_static_pointer();

    // load the function pointer from static table data
    let byte_offset = method_index as usize * Word::BYTE_LEN;
    let entry_address = vtable_pointer.address() + byte_offset;
    let function = unsafe { *(entry_address as *const Word) };
    let function = function.as_function_pointer();
    let function = mir::LocalNodeId::new(function.function_index());

    Ok(function)
}

/// Resolve the callee for one interface call.
fn resolve_interface_callee(
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

    // load the itab entry for the interface call
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

/// Resolve the callee for one indirect call.
fn resolve_indirect_callee(
    state: &mut DispatchState<'_, '_>,
    callable: Word,
    callable_type: mir::LocalNodeId<mir::Type>,
) -> Result<(mir::LocalNodeId<mir::Function>, Option<Word>), Error> {
    // use the actual callee SSA type, not the code signature
    let callable_type = repr_type(state.tree(), callable_type);
    match state.tree().get(callable_type) {
        mir::Type::FunctionPointer { .. } => {
            let function = mir::LocalNodeId::new(callable.as_function_pointer().function_index());

            Ok((function, None))
        }
        mir::Type::Callable { .. } => {
            let (function, environment_value) = callable::decode_callable(state, callable)?;
            let function = mir::LocalNodeId::new(function.as_function_pointer().function_index());

            Ok((function, Some(environment_value)))
        }
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return the expected signature for one indirect callable type.
fn indirect_callable_signature(
    tree: &mir::Tree,
    callable_type: mir::LocalNodeId<mir::Type>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let callable_type = repr_type(tree, callable_type);
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

    if function.return_type != *result {
        return Err(Error::TypeMismatch {
            expected: format!("function signature {signature:?}"),
            actual: format!("function {function_id:?}"),
        });
    }

    Ok(())
}

/// Require one lowered call target for a function id.
#[inline]
fn require_call_target(
    state: &DispatchState<'_, '_>,
    function: mir::LocalNodeId<mir::Function>,
) -> Result<CallTarget, Error> {
    state
        .program
        .functions
        .resolve(function)
        .ok_or(Error::UndefinedFunction { function })
}

/// Load a function pointer.
pub(crate) fn execute_address_function(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let FunctionAddr { dest, function } = instruction.payload_as::<FunctionAddr>();

    // build function pointer value
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let value = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));

    // store result
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Build a callable value from one function and environment.
pub(crate) fn execute_bind_callable(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallableBind {
        dest,
        function,
        environment,
    } = instruction.payload_as::<CallableBind>();

    // bind the function pointer and environment into a callable object
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
    let ty = match state.value_type(*dest) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let value = match callable::bind_callable(state, ty, function, *environment) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Load the callable environment pointer for the current frame.
pub(crate) fn execute_load_callable_environment(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let CallableEnvironment { dest } = instruction.payload_as::<CallableEnvironment>();

    // load current frame environment
    let frame_layout = state.frame_layout() as *const engine::FrameLayout;
    let environment = match state
        .current_frame_mut()
        .environment(unsafe { &*frame_layout })
    {
        Ok(environment) => environment,
        Err(error) => return Transfer::Error(error),
    };
    let Some(environment) = environment else {
        return Transfer::Error(Error::InvalidInstruction);
    };

    // store result
    state.set_word(*dest, environment);

    // continue to next instruction
    Transfer::Continue
}

/// Return one local callee for a direct call.
#[inline]
fn direct_callee(state: &DispatchState<'_, '_>, target: CallTarget) -> Option<NonNull<Function>> {
    // only local callees can enter directly
    match target {
        CallTarget::Local(index) => state.program.functions.pointer(index),
        CallTarget::Import => None,
    }
}

/// Try to enter one lowered callee without creating a call transfer.
#[inline]
fn try_enter_direct_call(
    state: &mut DispatchState<'_, '_>,
    target: CallTarget,
    env: Option<Word>,
    move_plan: Option<MoveRange>,
    resume_pc: usize,
) -> Option<Transfer> {
    // NOTE #Performance: avoid call transfers for hot lowered calls
    let move_plan = move_plan?;

    // require one local callee before entering
    let callee_ptr = direct_callee(state, target)?;
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
        callee_ptr,
        entry_block_ptr,
        layout,
        stack_offset,
        frame_base,
    );
    new_frame.set_environment(layout, env);

    // bind parameters from the current caller frame
    let caller_index = state.frame_index;
    let current_function_ptr = {
        let frame = state.current_frame_mut();
        frame.function_ptr
    };
    let current_function = unsafe { current_function_ptr.as_ref() };
    let caller_ptr = {
        let Ok(caller) = state.frame(caller_index) else {
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
    Some(dispatch_block(state, entry_instructions, 0))
}

/// Enter one call or return a call transfer.
fn enter_call(
    state: &mut DispatchState<'_, '_>,
    dest: Option<mir::Value>,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
    move_plan: Option<MoveRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> Transfer {
    // enter local callees without bouncing through transfer handling
    if allow_direct
        && let Some(transfer) = try_enter_direct_call(state, target, env, move_plan, resume_pc)
    {
        return transfer;
    }

    // otherwise return the call to transfer handling
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

/// Build one call branch transfer.
fn call_branch_transfer(
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
    normal_state: engine::FrameStateId,
    unwind_state: engine::FrameStateId,
) -> Transfer {
    // exceptional calls always use explicit transfer handling
    Transfer::CallBranch {
        function: function_id.id,
        target,
        arguments,
        env,
        normal_state,
        unwind_state,
    }
}

/// Execute direct function call.
pub(crate) fn execute_call(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Call {
        dest,
        function,
        target,
        arguments,
        moves,
    } = instruction.payload_as::<Call>();

    // load target function id
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let move_plan = Some(*moves);

    // skip direct call when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

    enter_call(
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
pub(crate) fn execute_invoke(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallBranch {
        function,
        target,
        arguments,
        normal_state,
        unwind_state,
    } = instruction.payload_as::<CallBranch>();

    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);

    call_branch_transfer(
        function_id,
        *target,
        *arguments,
        None,
        *normal_state,
        *unwind_state,
    )
}

/// Execute virtual function call.
pub(crate) fn execute_call_virtual(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let CallVirtual {
        dest,
        receiver,
        table_field,
        method_index,
        arguments,
    } = instruction.payload_as::<CallVirtual>();

    // resolve dynamic callee
    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_virtual_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

    enter_call(
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
pub(crate) fn execute_invoke_virtual(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallVirtualBranch {
        receiver,
        table_field,
        method_index,
        arguments,
        normal_state,
        unwind_state,
    } = instruction.payload_as::<CallVirtualBranch>();

    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_virtual_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(
        function_id,
        target,
        *arguments,
        None,
        *normal_state,
        *unwind_state,
    )
}

/// Execute interface function call.
pub(crate) fn execute_call_interface(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let CallInterface {
        dest,
        receiver,
        table_field,
        method_index,
        arguments,
    } = instruction.payload_as::<CallInterface>();

    // resolve dynamic callee
    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_interface_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = state.options().limits.max_instructions.is_none();

    enter_call(
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
pub(crate) fn execute_invoke_interface(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallInterfaceBranch {
        receiver,
        table_field,
        method_index,
        arguments,
        normal_state,
        unwind_state,
    } = instruction.payload_as::<CallInterfaceBranch>();

    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_interface_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(
        function_id,
        target,
        *arguments,
        None,
        *normal_state,
        *unwind_state,
    )
}

/// Execute indirect function call.
pub(crate) fn execute_call_indirect(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let CallIndirect {
        dest,
        callee,
        arguments,
    } = instruction.payload_as::<CallIndirect>();

    // load callee value
    let callee_val = state.get(*callee);

    // resolve callable function and environment
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callee(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }

    // load the lowered call target
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // return control to transfer handling
    Transfer::Call {
        function,
        target,
        destination: *dest,
        arguments: *arguments,
        env,
        moves: None,
        resume_pc: pc + 1,
    }
}

/// Execute exceptional indirect call terminator.
pub(crate) fn execute_invoke_indirect(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallIndirectBranch {
        callee,
        arguments,
        normal_state,
        unwind_state,
    } = instruction.payload_as::<CallIndirectBranch>();

    let callee_val = state.get(*callee);
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callee(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(
        function_id,
        target,
        *arguments,
        env,
        *normal_state,
        *unwind_state,
    )
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    state: &mut DispatchState<'_, '_>,
    callee: &Function,
    argument_values: &[FrameValue],
    env: Option<Word>,
) -> Result<(), Error> {
    // update frame cache
    let entry_block = &callee.blocks[callee.entry as usize];
    let layout = state
        .program
        .frame_layout_by_id(callee.frame_layout)
        .ok_or(Error::InvalidInstruction)?;

    // replace the current frame bytes in place
    let stack_offset = state.current_frame_mut().stack_offset;
    state.engine.truncate_stack(stack_offset);
    let (stack_offset, frame_base) = state
        .engine
        .allocate_frame(layout, state.options)
        .map_err(|error| error.error)?;
    {
        let frame = state.current_frame_mut();
        frame.frame_layout = callee.frame_layout;
        frame.function_ptr = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.resume_pc = 0;
        frame.replace_bytes(stack_offset, layout.byte_len as usize, frame_base);
        frame.set_environment(layout, env);
    }

    // refresh cached pointers for the new function
    state.refresh_frame(callee);

    // bind function parameters
    let program = state.program;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    store_parameters(
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TailCall {
        function,
        target,
        moves,
    } = instruction.payload_as::<TailCall>();

    // require the local callee
    let resolved_index = match *target {
        CallTarget::Local(index) => Some(index),
        CallTarget::Import => None,
    };

    // return imported calls to transfer handling
    let Some(resolved_index) = resolved_index else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
        };
    };
    let Some(callee_ptr) = state.program.functions.pointer(resolved_index) else {
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
        let caller = match state.frame(state.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_planned_arguments(
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

    // enter tail call
    if let Err(error) = enter_tail_call(state, callee, &argument_values, None) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    dispatch_block(state, entry_instructions, 0)
}

/// Execute self tail call by reusing the current frame.
pub(crate) fn execute_tail_call_self(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TailCallSelf { entry, arguments } = instruction.payload_as::<TailCallSelf>();

    // load current function entry block
    let function_ptr = {
        let frame = state.current_frame_mut();
        frame.function_ptr
    };
    let function = unsafe { function_ptr.as_ref() };
    let entry_block = &function.blocks[*entry as usize];

    // collect argument values before clearing the frame
    let args = {
        let caller = match state.frame(state.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_arguments(
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
    let frame_layout = unsafe { &*frame_layout };

    // discard stack allocations from the previous self call
    {
        let (stack_offset, frame_base) = {
            let frame = state.current_frame_mut();
            (frame.stack_offset, frame.base_address() as *mut u8)
        };
        let frame_byte_len = frame_layout.byte_len as usize;

        state.engine.truncate_stack(stack_offset + frame_byte_len);
        let frame = state.current_frame_mut();
        frame.replace_bytes(stack_offset, frame_byte_len, frame_base);
        frame.clear_values(frame_layout);
    }

    // update frame to entry block
    {
        let frame = state.current_frame_mut();
        frame.block_ptr = NonNull::from(entry_block);
        frame.resume_pc = 0;
    }

    // refresh cached frame pointers after replacing frame bytes
    state.refresh_frame(function);

    // bind function parameters
    let program = state.program;
    let frame_ptr = state.current_frame_mut() as *mut Frame;
    let frame = unsafe { &mut *frame_ptr };
    if let Err(error) = store_parameters(
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
    dispatch_block(state, entry_instructions, 0)
}

/// Execute virtual tail call.
pub(crate) fn execute_tail_call_virtual(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TailCallVirtual {
        receiver,
        table_field,
        method_index,
        arguments,
    } = instruction.payload_as::<TailCallVirtual>();

    // resolve dynamic callee
    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_virtual_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::TailCall {
        function: function_id.id,
        target,
        arguments: *arguments,
        env: None,
        moves: None,
    }
}

/// Execute interface tail call.
pub(crate) fn execute_tail_call_interface(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TailCallInterface {
        receiver,
        table_field,
        method_index,
        arguments,
    } = instruction.payload_as::<TailCallInterface>();

    // resolve dynamic callee
    let receiver_value = state.get(*receiver);
    let table_field = load_table_field(state, *table_field);
    let function_id =
        match resolve_interface_callee(state, receiver_value, table_field, *method_index) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TailCallIndirect { callee, arguments } = instruction.payload_as::<TailCallIndirect>();

    // load callee value
    let callee_val = state.get(*callee);

    // resolve callable function and environment
    let callee_type = match state.value_type(*callee) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let signature = match indirect_callable_signature(state.tree(), callee_type) {
        Ok(signature) => signature,
        Err(error) => return Transfer::Error(error),
    };
    let (function_id, env) = match resolve_indirect_callee(state, callee_val, callee_type) {
        Ok(resolved) => resolved,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = validate_indirect_signature(state.tree(), function_id, signature) {
        return Transfer::Error(error);
    }

    // load the lowered call target
    let target = match require_call_target(state, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // return imported calls to transfer handling
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
    let Some(callee_ptr) = state.program.functions.pointer(resolved_index) else {
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
    let caller = match state.frame(state.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = unsafe { caller.function_ptr.as_ref() };
    let argument_values = match load_arguments(
        state.program,
        state.engine.frames.as_slice(),
        caller,
        caller_function.argument_pool.as_slice(),
        *arguments,
    ) {
        Ok(arguments) => arguments,
        Err(error) => return Transfer::Error(error),
    };

    // enter tail call
    if let Err(error) = enter_tail_call(state, callee, &argument_values, env) {
        return Transfer::Error(error);
    }

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    dispatch_block(state, entry_instructions, 0)
}
