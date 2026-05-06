use std::ptr::NonNull;

use destack_engine as engine;

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_values, store_parameters,
};
use super::{access, callable};
use crate::diagnostic::Error;
use crate::interpreter::{Frame, Machine};
use crate::program::{
    ArgumentRange, Call, CallBranch, CallIndirect, CallIndirectBranch, CallInterface,
    CallInterfaceBranch, CallTarget, CallVirtual, CallVirtualBranch, CallableBind, Function,
    Instruction, MoveRange, Projection, TailCall, TailCallIndirect, TailCallInterface,
    TailCallVirtual, Transfer,
};
use crate::{FunctionPointer, Word};
use destack_mir as mir;

/// Load one lowered call table field from a receiver.
fn load_receiver_field<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    receiver: Word,
    field: Projection,
) -> Result<Word, Error> {
    if IS_SHARED {
        return Ok(access::load_shared_heap_scalar::<8, false>(
            machine,
            receiver,
            field.byte_offset,
        ));
    }

    Ok(access::load_heap_scalar::<8, false>(
        machine,
        receiver,
        field.byte_offset,
    ))
}

/// Resolve the callee for one virtual call.
fn resolve_virtual_callee<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    receiver: Word,
    table_field: Projection,
    method_index: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field::<IS_SHARED>(machine, receiver, table_field)?;
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
fn resolve_interface_callee<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    receiver: Word,
    table_field: Projection,
    method_index: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the itab id from the interface reference
    let itab_value = load_receiver_field::<IS_SHARED>(machine, receiver, table_field)?;

    // decode the itab id
    let raw_id = itab_value.as_u64();
    let raw_id = u32::try_from(raw_id).map_err(|_| Error::InvalidInstruction)?;
    let table_id = mir::ItabId::new(raw_id);

    // load the lowered interface method target
    machine
        .program
        .interface_method(table_id, method_index)
        .ok_or(Error::InvalidInstruction)
}

/// Resolve the callee for one indirect call.
fn resolve_indirect_callee<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    callable: Word,
) -> Result<(mir::LocalNodeId<mir::Function>, Option<Word>), Error> {
    // function pointers are already the callee payload
    if !HAS_ENVIRONMENT {
        let function = mir::LocalNodeId::new(callable.as_function_pointer().function_index());

        return Ok((function, None));
    }

    // callable values carry a function pointer and environment pointer
    let (function, environment_value) = callable::decode_callable(machine, callable)?;
    let function = mir::LocalNodeId::new(function.as_function_pointer().function_index());

    Ok((function, Some(environment_value)))
}

/// Require one lowered call target for a function id.
#[inline]
fn require_call_target(
    machine: &Machine<'_, '_>,
    function: mir::LocalNodeId<mir::Function>,
) -> Result<CallTarget, Error> {
    machine
        .program
        .functions
        .resolve(function)
        .ok_or(Error::UndefinedFunction { function })
}

/// Load a function pointer.
pub(crate) fn execute_address_function(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // build function pointer value
    let dest = instruction.a;
    let function_id = mir::LocalNodeId::<mir::Function>::new(instruction.b);
    let value = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));

    // store result
    machine.set_word_at(dest, value);

    Ok(())
}

/// Build a callable value from one function and word environment.
pub(crate) fn execute_bind_callable_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let function = instruction.b;
    let environment_offset = instruction.c;
    let CallableBind {
        callable_layout,
        object_layout,
        environment,
    } = *machine.side_with_id::<CallableBind>(instruction.d);

    // bind the function pointer and environment into a callable object
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
    let value = callable::bind_callable(
        machine,
        callable_layout,
        object_layout,
        function,
        environment,
        environment_offset,
    )?;

    // store result
    machine.set_word_at(dest_offset, value);

    Ok(())
}

/// Build a callable value from one function and frame address environment.
pub(crate) fn execute_bind_callable_address(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let function = instruction.b;
    let environment_offset = instruction.c;
    let CallableBind {
        callable_layout,
        object_layout,
        environment,
    } = *machine.side_with_id::<CallableBind>(instruction.d);

    // bind the function pointer and environment into a callable object
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
    let value = callable::bind_callable(
        machine,
        callable_layout,
        object_layout,
        function,
        environment,
        environment_offset,
    )?;

    // store result
    machine.set_word_at(dest_offset, value);

    Ok(())
}

/// Load the callable environment pointer for the current frame.
pub(crate) fn execute_load_callable_environment(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;

    // load current frame environment
    let frame_layout = machine.frame_layout() as *const engine::FrameLayout;
    let environment = machine
        .current_frame_mut()
        .environment(unsafe { &*frame_layout })?;
    let Some(environment) = environment else {
        return Err(Error::InvalidInstruction);
    };

    // store result
    machine.set_word_at(dest, environment);

    Ok(())
}

/// Return one local function for a call target.
#[inline]
fn local_function(machine: &Machine<'_, '_>, target: CallTarget) -> Option<NonNull<Function>> {
    // only local callees can enter directly
    match target {
        CallTarget::Local(index) => machine.program.functions.pointer(index),
        CallTarget::Import => None,
    }
}

/// Enter one local callee without creating a call transfer.
#[inline]
fn enter_local_call(
    machine: &mut Machine<'_, '_>,
    target: CallTarget,
    env: Option<Word>,
    moves: Option<MoveRange>,
    resume_pc: usize,
) -> Option<Transfer> {
    // NOTE #Performance: avoid transfer plumbing for local calls
    let moves = moves?;

    // require one local callee before entering
    let callee_ptr = local_function(machine, target)?;
    let callee = unsafe { callee_ptr.as_ref() };

    // reject stack overflow before mutating any live machine
    if machine.interpreter.frames.len() >= machine.options().limits.max_stack_depth {
        return Some(Transfer::Error(Error::StackOverflow));
    }

    // store the caller pc before allocating the callee
    {
        let caller = machine.current_frame_mut();
        caller.pc = resume_pc;
    }

    let layout = match machine.program.frame_layout_by_id(callee.frame_layout) {
        Some(layout) => layout,
        None => return Some(Transfer::Error(Error::InvalidInstruction)),
    };
    let (stack_offset, frame_base) = match machine.interpreter.allocate_frame(layout) {
        Ok(frame) => frame,
        Err(error) => return Some(Transfer::Error(error.error)),
    };
    let mut new_frame = Frame::new(
        callee.frame_layout,
        callee_ptr,
        callee.entry,
        layout,
        stack_offset,
        frame_base,
    );
    new_frame.set_environment(layout, env);

    // bind parameters from the current caller frame
    let caller_index = machine.frame_index;
    let current_function_ptr = {
        let frame = machine.current_frame_mut();
        frame.function_ptr
    };
    let current_function = unsafe { current_function_ptr.as_ref() };
    let caller_ptr = {
        let Ok(caller) = machine.frame(caller_index) else {
            return Some(Transfer::Error(Error::InvalidHeapReference));
        };
        caller as *const Frame
    };
    let caller = unsafe { &*caller_ptr };

    if let Err(error) = move_values(
        caller,
        &mut new_frame,
        moves,
        current_function.move_pool.as_slice(),
    ) {
        return Some(Transfer::Error(error));
    }

    // push the callee frame and continue at its entry block
    machine.interpreter.frames.push(new_frame);
    let new_index = machine.interpreter.frames.len() - 1;
    if let Err(error) = machine.enter_frame(new_index, callee) {
        return Some(Transfer::Error(error));
    }

    Some(Transfer::Enter)
}

/// Enter one call or return a call transfer.
fn enter_call(
    machine: &mut Machine<'_, '_>,
    dest: Option<mir::Value>,
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
    moves: Option<MoveRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> Transfer {
    // enter local callees without bouncing through transfer handling
    if allow_direct && let Some(transfer) = enter_local_call(machine, target, env, moves, resume_pc)
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
        moves,
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
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let Call {
        dest,
        function,
        target,
        arguments,
        moves,
    } = machine.side::<Call>(instruction);

    // load target function id
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let moves = Some(*moves);

    // skip direct call when instruction limits are active
    let allow_direct = machine.options().limits.max_instructions.is_none();

    enter_call(
        machine,
        *dest,
        function_id,
        *target,
        *arguments,
        None,
        moves,
        pc + 1,
        allow_direct,
    )
}

/// Execute exceptional direct call terminator.
pub(crate) fn execute_invoke(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let CallBranch {
        function,
        target,
        arguments,
        normal_state,
        unwind_state,
    } = machine.side::<CallBranch>(instruction);

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

/// Execute a virtual function call with a statically known receiver heap.
fn execute_call_virtual<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallVirtual {
        dest,
        receiver_offset,
        table_field,
        method_index,
        arguments,
    } = machine.side::<CallVirtual>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_virtual_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = machine.options().limits.max_instructions.is_none();

    enter_call(
        machine,
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

/// Execute virtual function call through a local heap receiver.
pub(crate) fn execute_call_virtual_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<false>(machine, instruction, pc)
}

/// Execute virtual function call through a shared heap receiver.
pub(crate) fn execute_call_virtual_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<true>(machine, instruction, pc)
}

/// Execute an exceptional virtual call with a statically known receiver heap.
fn execute_invoke_virtual<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallVirtualBranch {
        receiver_offset,
        table_field,
        method_index,
        arguments,
        normal_state,
        unwind_state,
    } = machine.side::<CallVirtualBranch>(instruction);

    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_virtual_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
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

/// Execute exceptional virtual call through a local heap receiver.
pub(crate) fn execute_invoke_virtual_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_virtual::<false>(machine, instruction)
}

/// Execute exceptional virtual call through a shared heap receiver.
pub(crate) fn execute_invoke_virtual_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_virtual::<true>(machine, instruction)
}

/// Execute an interface function call with a statically known receiver heap.
fn execute_call_interface<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallInterface {
        dest,
        receiver_offset,
        table_field,
        method_index,
        arguments,
    } = machine.side::<CallInterface>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_interface_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = machine.options().limits.max_instructions.is_none();

    enter_call(
        machine,
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

/// Execute interface function call through a local heap receiver.
pub(crate) fn execute_call_interface_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_interface::<false>(machine, instruction, pc)
}

/// Execute interface function call through a shared heap receiver.
pub(crate) fn execute_call_interface_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_interface::<true>(machine, instruction, pc)
}

/// Execute an exceptional interface call with a statically known receiver heap.
fn execute_invoke_interface<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallInterfaceBranch {
        receiver_offset,
        table_field,
        method_index,
        arguments,
        normal_state,
        unwind_state,
    } = machine.side::<CallInterfaceBranch>(instruction);

    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_interface_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
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

/// Execute exceptional interface call through a local heap receiver.
pub(crate) fn execute_invoke_interface_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_interface::<false>(machine, instruction)
}

/// Execute exceptional interface call through a shared heap receiver.
pub(crate) fn execute_invoke_interface_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_interface::<true>(machine, instruction)
}

/// Execute an indirect call with a statically known callee shape.
fn execute_indirect_call<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallIndirect {
        dest,
        callee_offset,
        signature,
        arguments,
    } = machine.side::<CallIndirect>(instruction);

    // load callee value
    let callee_value = machine.get_word_at(*callee_offset);

    // resolve callable function and environment
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = machine
        .program
        .functions
        .validate_signature(function_id, *signature)
    {
        return Transfer::Error(error);
    }

    // load the lowered call target
    let target = match require_call_target(machine, function_id) {
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

/// Execute indirect function call.
pub(crate) fn execute_call_indirect(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_indirect_call::<false>(machine, instruction, pc)
}

/// Execute callable value call.
pub(crate) fn execute_call_callable(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_indirect_call::<true>(machine, instruction, pc)
}

/// Execute an exceptional indirect call with a statically known callee shape.
fn execute_indirect_invoke<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallIndirectBranch {
        callee_offset,
        signature,
        arguments,
        normal_state,
        unwind_state,
    } = machine.side::<CallIndirectBranch>(instruction);

    let callee_value = machine.get_word_at(*callee_offset);
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = machine
        .program
        .functions
        .validate_signature(function_id, *signature)
    {
        return Transfer::Error(error);
    }
    let target = match require_call_target(machine, function_id) {
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

/// Execute exceptional indirect call terminator.
pub(crate) fn execute_invoke_indirect(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_invoke::<false>(machine, instruction)
}

/// Execute exceptional callable call terminator.
pub(crate) fn execute_invoke_callable(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_invoke::<true>(machine, instruction)
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    machine: &mut Machine<'_, '_>,
    callee: &Function,
    argument_values: &[FrameValue],
    env: Option<Word>,
) -> Result<(), Error> {
    // load callee frame layout
    let layout = machine
        .program
        .frame_layout_by_id(callee.frame_layout)
        .ok_or(Error::InvalidInstruction)?;

    // replace the current frame bytes in place
    let stack_offset = machine.current_frame_mut().stack_offset;
    machine.interpreter.truncate_stack(stack_offset);
    let (stack_offset, frame_base) = machine
        .interpreter
        .allocate_frame(layout)
        .map_err(|error| error.error)?;
    {
        let frame = machine.current_frame_mut();
        frame.frame_layout = callee.frame_layout;
        frame.function_ptr = NonNull::from(callee);
        frame.block = callee.entry;
        frame.pc = 0;
        frame.replace_bytes(stack_offset, layout.byte_len as usize, frame_base);
        frame.set_environment(layout, env);
    }

    // refresh cached pointers for the new function
    machine.refresh_frame(callee)?;

    // bind function parameters
    let program = machine.program;
    let frame_ptr = machine.current_frame_mut() as *mut Frame;
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
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCall {
        function,
        target,
        moves,
    } = machine.side::<TailCall>(instruction);

    // require the local callee
    let local_index = match *target {
        CallTarget::Local(index) => Some(index),
        CallTarget::Import => None,
    };

    // return imported calls to transfer handling
    let Some(local_index) = local_index else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
        };
    };
    let Some(callee_ptr) = machine.program.functions.pointer(local_index) else {
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
        let function_ptr = machine.current_frame_mut().function_ptr;
        let current_func = unsafe { function_ptr.as_ref() };
        let caller = match machine.frame(machine.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_moved_arguments(
            machine.program,
            caller,
            current_func.move_pool.as_slice(),
            *moves,
        ) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };

    // enter tail call
    if let Err(error) = enter_tail_call(machine, callee, &argument_values, None) {
        return Transfer::Error(error);
    }

    Transfer::Enter
}

/// Execute self tail call by reusing the current frame.
pub(crate) fn execute_tail_call_self(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let entry = instruction.a;
    let arguments = ArgumentRange {
        start: instruction.b,
        len: instruction.c,
    };

    // load current function entry block
    let function_ptr = {
        let frame = machine.current_frame_mut();
        frame.function_ptr
    };
    let function = unsafe { function_ptr.as_ref() };

    // collect argument values before clearing the frame
    let args = {
        let caller = match machine.frame(machine.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_arguments(
            machine.program,
            machine.interpreter.frames.as_slice(),
            caller,
            function.argument_pool.as_slice(),
            arguments,
        ) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };
    let frame_layout = machine.frame_layout() as *const engine::FrameLayout;
    let frame_layout = unsafe { &*frame_layout };

    // discard stack allocations from the previous self call
    {
        let (stack_offset, frame_base) = {
            let frame = machine.current_frame_mut();
            (frame.stack_offset, frame.base_address() as *mut u8)
        };
        let frame_byte_len = frame_layout.byte_len as usize;

        machine
            .interpreter
            .truncate_stack(stack_offset + frame_byte_len);
        let frame = machine.current_frame_mut();
        frame.replace_bytes(stack_offset, frame_byte_len, frame_base);
        frame.clear_values(frame_layout);
    }

    // update frame to entry block
    {
        let frame = machine.current_frame_mut();
        frame.block = entry;
        frame.pc = 0;
    }

    // refresh cached frame pointers after replacing frame bytes
    if let Err(error) = machine.refresh_frame(function) {
        return Transfer::Error(error);
    }

    // bind function parameters
    let program = machine.program;
    let frame_ptr = machine.current_frame_mut() as *mut Frame;
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

    Transfer::Enter
}

/// Execute indirect tail call.
pub(crate) fn execute_tail_call_indirect(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_tail_call::<false>(machine, instruction)
}

/// Execute callable value tail call.
pub(crate) fn execute_tail_call_callable(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_tail_call::<true>(machine, instruction)
}

/// Execute a virtual tail call with a statically known receiver heap.
fn execute_tail_call_virtual<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCallVirtual {
        receiver_offset,
        table_field,
        method_index,
        arguments,
    } = machine.side::<TailCallVirtual>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_virtual_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
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

/// Execute virtual tail call through a local heap receiver.
pub(crate) fn execute_tail_call_virtual_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_virtual::<false>(machine, instruction)
}

/// Execute virtual tail call through a shared heap receiver.
pub(crate) fn execute_tail_call_virtual_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_virtual::<true>(machine, instruction)
}

/// Execute an interface tail call with a statically known receiver heap.
fn execute_tail_call_interface<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCallInterface {
        receiver_offset,
        table_field,
        method_index,
        arguments,
    } = machine.side::<TailCallInterface>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.get_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id = match resolve_interface_callee::<IS_SHARED>(
        machine,
        receiver_value,
        table_field,
        *method_index,
    ) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(machine, function_id) {
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

/// Execute interface tail call through a local heap receiver.
pub(crate) fn execute_tail_call_interface_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_interface::<false>(machine, instruction)
}

/// Execute interface tail call through a shared heap receiver.
pub(crate) fn execute_tail_call_interface_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_interface::<true>(machine, instruction)
}

/// Execute an indirect tail call with a statically known callee shape.
fn execute_indirect_tail_call<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCallIndirect {
        callee_offset,
        signature,
        arguments,
    } = machine.side::<TailCallIndirect>(instruction);

    // load callee value
    let callee_value = machine.get_word_at(*callee_offset);

    // resolve callable function and environment
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) = machine
        .program
        .functions
        .validate_signature(function_id, *signature)
    {
        return Transfer::Error(error);
    }

    // load the lowered call target
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // return imported calls to transfer handling
    let local_index = match target {
        CallTarget::Local(index) => Some(index),
        CallTarget::Import => None,
    };
    let Some(local_index) = local_index else {
        return Transfer::TailCall {
            function,
            target,
            arguments: *arguments,
            env,
            moves: None,
        };
    };
    let Some(callee_ptr) = machine.program.functions.pointer(local_index) else {
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
    let caller = match machine.frame(machine.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = unsafe { caller.function_ptr.as_ref() };
    let argument_values = match load_arguments(
        machine.program,
        machine.interpreter.frames.as_slice(),
        caller,
        caller_function.argument_pool.as_slice(),
        *arguments,
    ) {
        Ok(arguments) => arguments,
        Err(error) => return Transfer::Error(error),
    };

    // enter tail call
    if let Err(error) = enter_tail_call(machine, callee, &argument_values, env) {
        return Transfer::Error(error);
    }

    Transfer::Enter
}
