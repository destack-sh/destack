use destack_engine as engine;

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_values, store_parameters,
};
use super::{access, closure};
use crate::diagnostic::Error;
use crate::interpreter::{Frame, Machine};
use crate::program::{
    ArgumentRange, Call, CallBranch, CallDynamic, CallDynamicBranch, CallIndirect,
    CallIndirectBranch, CallTarget, CallVirtual, CallVirtualBranch, ClosureBind, Function,
    Instruction, MoveRange, Projection, TailCall, TailCallDynamic, TailCallIndirect,
    TailCallVirtual, Transfer, WordLayout,
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

/// Load one function target from an immutable dispatch table.
fn load_dispatch_slot(
    machine: &Machine<'_, '_>,
    table_pointer: engine::StaticPointer,
    slot: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // dispatch table slots are target pointers
    let pointer_bytes = machine.program.tree.pointer_bytes() as usize;
    let byte_offset = slot as usize * pointer_bytes;
    let entry_pointer = table_pointer.add_bytes(byte_offset);

    // static dispatch tables contain function addresses
    let function = load_function_pointer(entry_pointer.address(), pointer_bytes)?;
    let function = function.as_function_pointer();
    let function = mir::LocalNodeId::new(function.function_index());

    Ok(function)
}

/// Load one function address from static memory.
fn load_function_pointer(address: usize, pointer_bytes: usize) -> Result<Word, Error> {
    // read static bytes without assuming stronger alignment
    let raw = unsafe {
        match pointer_bytes {
            4 => u32::from_le((address as *const u32).read_unaligned()) as u64,
            8 => u64::from_le((address as *const u64).read_unaligned()),
            _ => return Err(Error::invalid_instruction()),
        }
    };

    Ok(WordLayout::FunctionPointer.decode(raw))
}

/// Resolve the callee for one virtual call.
fn resolve_virtual_callee<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    receiver: Word,
    table_field: Projection,
    slot: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field::<IS_SHARED>(machine, receiver, table_field)?;
    let vtable_pointer = vtable_value.as_static_pointer();

    load_dispatch_slot(machine, vtable_pointer, slot)
}

/// Resolve the callee for one dynamic call.
fn resolve_dynamic_callee<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    receiver: Word,
    table_field: Projection,
    slot: u32,
) -> Result<mir::LocalNodeId<mir::Function>, Error> {
    // load the dynamic table pointer from the erased receiver
    let dynamic_table_value = load_receiver_field::<IS_SHARED>(machine, receiver, table_field)?;
    let dynamic_table_pointer = dynamic_table_value.as_static_pointer();

    load_dispatch_slot(machine, dynamic_table_pointer, slot)
}

/// Resolve the callee for one indirect call.
fn resolve_indirect_callee<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    callee: Word,
) -> Result<(mir::LocalNodeId<mir::Function>, Option<Word>), Error> {
    // function pointers are already the callee payload
    if !HAS_ENVIRONMENT {
        let function = mir::LocalNodeId::new(callee.as_function_pointer().function_index());

        return Ok((function, None));
    }

    // closure values carry a function pointer and environment pointer
    let (function, environment_value) = closure::decode_closure(machine, callee)?;
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
        .call_target(function)
        .ok_or(Error::undefined_function(function))
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
    machine.store_word_at(dest, value);

    Ok(())
}

/// Build a closure value from one function and word environment.
pub(crate) fn execute_bind_closure_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let function = instruction.b;
    let environment_offset = instruction.c;
    let ClosureBind {
        closure_layout,
        object_layout,
        environment,
    } = *machine.side_record::<ClosureBind>(instruction.d);

    // bind the function pointer and environment into a closure object
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
    let value = closure::bind_closure(
        machine,
        closure_layout,
        object_layout,
        function,
        environment,
        environment_offset,
    )?;

    // store result
    machine.store_word_at(dest_offset, value);

    Ok(())
}

/// Build a closure value from one function and frame address environment.
pub(crate) fn execute_bind_closure_address(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let function = instruction.b;
    let environment_offset = instruction.c;
    let ClosureBind {
        closure_layout,
        object_layout,
        environment,
    } = *machine.side_record::<ClosureBind>(instruction.d);

    // bind the function pointer and environment into a closure object
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let function = Word::function_pointer(FunctionPointer::from_bits(function_id.id as usize));
    let value = closure::bind_closure(
        machine,
        closure_layout,
        object_layout,
        function,
        environment,
        environment_offset,
    )?;

    // store result
    machine.store_word_at(dest_offset, value);

    Ok(())
}

/// Load the closure environment pointer for the current frame.
pub(crate) fn execute_load_closure_environment(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;

    // load current frame environment
    let environment = machine
        .active_frame()
        .load_environment(machine.frame_layout())?;
    let Some(environment) = environment else {
        return Err(Error::invalid_instruction());
    };

    // store result
    machine.store_word_at(dest, environment);

    Ok(())
}

/// Return one local function for a call target.
#[inline]
fn local_function<'iso>(machine: &Machine<'_, 'iso>, target: CallTarget) -> Option<&'iso Function> {
    // only local callees can enter directly
    match target {
        CallTarget::Local(index) => machine.program.functions.function_by_index(index),
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
    let callee = local_function(machine, target)?;

    // reject stack overflow before mutating any live machine
    if machine.interpreter.frames.len() >= machine.options().limits.max_stack_depth {
        return Some(Transfer::Error(Error::stack_overflow()));
    }

    // store the caller pc before allocating the callee
    {
        let caller = machine.active_frame_mut();
        caller.pc = resume_pc;
    }

    let layout = match machine.program.frame_layout_by_id(callee.frame_layout) {
        Some(layout) => layout,
        None => return Some(Transfer::Error(Error::invalid_instruction())),
    };
    let (stack_offset, frame_base) = match machine.interpreter.allocate_frame(layout) {
        Ok(frame) => frame,
        Err(error) => return Some(Transfer::Error(error.error)),
    };
    let mut new_frame = Frame::new(callee, callee.entry, layout, stack_offset, frame_base);
    if let Err(error) = new_frame.store_environment(layout, env) {
        return Some(Transfer::Error(error));
    }

    // bind parameters from the current caller frame
    let caller_index = machine.frame_index;
    let current_function = {
        let frame = machine.active_frame();
        match machine.program.functions.function_by_id(frame.function()) {
            Some(function) => function,
            None => return Some(Transfer::Error(Error::invalid_instruction())),
        }
    };
    let caller = match machine.frame(caller_index) {
        Ok(caller) => caller,
        Err(error) => return Some(Transfer::Error(error)),
    };

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
    if let Err(error) = machine.enter_frame(new_index) {
        return Some(Transfer::Error(error));
    }

    Some(Transfer::Enter)
}

/// Enter one call or return a call transfer.
fn enter_call(
    machine: &mut Machine<'_, '_>,
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
        arguments,
        env,
        moves,
        resume_pc,
    }
}

/// Build one call terminator transfer.
fn call_branch_transfer(
    function_id: mir::LocalNodeId<mir::Function>,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Word>,
    target_state: engine::FrameStateId,
) -> Transfer {
    // call terminators always use explicit transfer handling
    Transfer::CallBranch {
        function: function_id.id,
        target,
        arguments,
        env,
        target_state,
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
        function_id,
        *target,
        *arguments,
        None,
        moves,
        pc + 1,
        allow_direct,
    )
}

/// Execute direct call terminator.
pub(crate) fn execute_call_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallBranch {
        function,
        target,
        arguments,
        target_state,
    } = machine.side::<CallBranch>(instruction);

    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);

    call_branch_transfer(function_id, *target, *arguments, None, *target_state)
}

/// Execute a class function call with a statically known receiver heap.
fn execute_call_virtual<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallVirtual {
        receiver_offset,
        table_field,
        slot,
        arguments,
    } = machine.side::<CallVirtual>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
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
        function_id,
        target,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Execute class function call through a local heap receiver.
pub(crate) fn execute_call_virtual_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<false>(machine, instruction, pc)
}

/// Execute class function call through a shared heap receiver.
pub(crate) fn execute_call_virtual_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<true>(machine, instruction, pc)
}

/// Execute a virtual call terminator with a statically known receiver heap.
fn execute_call_virtual_branch<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallVirtualBranch {
        receiver_offset,
        table_field,
        slot,
        arguments,
        target_state,
    } = machine.side::<CallVirtualBranch>(instruction);

    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(function_id, target, *arguments, None, *target_state)
}

/// Execute virtual call terminator through a local heap receiver.
pub(crate) fn execute_call_virtual_heap_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_call_virtual_branch::<false>(machine, instruction)
}

/// Execute virtual call terminator through a shared heap receiver.
pub(crate) fn execute_call_virtual_shared_heap_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_call_virtual_branch::<true>(machine, instruction)
}

/// Execute a dynamic function call with a statically known receiver heap.
fn execute_call_dynamic<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallDynamic {
        receiver_offset,
        table_field,
        slot,
        arguments,
    } = machine.side::<CallDynamic>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_dynamic_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
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
        function_id,
        target,
        *arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Execute dynamic function call through a local heap receiver.
pub(crate) fn execute_call_dynamic_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_dynamic::<false>(machine, instruction, pc)
}

/// Execute dynamic function call through a shared heap receiver.
pub(crate) fn execute_call_dynamic_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_dynamic::<true>(machine, instruction, pc)
}

/// Execute a dynamic call terminator with a statically known receiver heap.
fn execute_call_dynamic_branch<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallDynamicBranch {
        receiver_offset,
        table_field,
        slot,
        arguments,
        target_state,
    } = machine.side::<CallDynamicBranch>(instruction);

    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_dynamic_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(function_id, target, *arguments, None, *target_state)
}

/// Execute dynamic call terminator through a local heap receiver.
pub(crate) fn execute_call_dynamic_heap_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_call_dynamic_branch::<false>(machine, instruction)
}

/// Execute dynamic call terminator through a shared heap receiver.
pub(crate) fn execute_call_dynamic_shared_heap_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_call_dynamic_branch::<true>(machine, instruction)
}

/// Execute an indirect call with a statically known callee shape.
fn execute_indirect_call<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let CallIndirect {
        callee_offset,
        signature,
        arguments,
    } = machine.side::<CallIndirect>(instruction);

    // load callee value
    let callee_value = machine.load_word_at(*callee_offset);

    // resolve closure function and environment
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) =
        machine
            .program
            .functions
            .validate_signature(machine.tree(), function_id, *signature)
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

/// Execute closure value call.
pub(crate) fn execute_call_closure(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_indirect_call::<true>(machine, instruction, pc)
}

/// Execute an indirect call terminator with a statically known callee shape.
fn execute_indirect_call_branch<const HAS_ENVIRONMENT: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CallIndirectBranch {
        callee_offset,
        signature,
        arguments,
        target_state,
    } = machine.side::<CallIndirectBranch>(instruction);

    let callee_value = machine.load_word_at(*callee_offset);
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) =
        machine
            .program
            .functions
            .validate_signature(machine.tree(), function_id, *signature)
    {
        return Transfer::Error(error);
    }
    let target = match require_call_target(machine, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    call_branch_transfer(function_id, target, *arguments, env, *target_state)
}

/// Execute indirect call terminator.
pub(crate) fn execute_call_indirect_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_call_branch::<false>(machine, instruction)
}

/// Execute closure call terminator.
pub(crate) fn execute_call_closure_branch(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_call_branch::<true>(machine, instruction)
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call<'ctx, 'iso>(
    machine: &mut Machine<'ctx, 'iso>,
    callee: &'iso Function,
    argument_values: &[FrameValue],
    env: Option<Word>,
) -> Result<(), Error> {
    // load callee frame layout
    let layout = machine
        .program
        .frame_layout_by_id(callee.frame_layout)
        .ok_or(Error::invalid_instruction())?;

    // replace the current frame bytes in place
    let stack_offset = machine.active_frame_mut().stack_offset;
    machine.interpreter.truncate_stack(stack_offset);
    let (stack_offset, frame_base) = machine
        .interpreter
        .allocate_frame(layout)
        .map_err(|error| error.error)?;
    {
        let frame = machine.active_frame_mut();
        frame.retarget(
            callee,
            callee.entry,
            stack_offset,
            layout.byte_len as usize,
            frame_base,
        );
        frame.store_environment(layout, env)?;
    }

    // load dispatch metadata for the retargeted frame
    machine.load_active_frame()?;

    // bind function parameters
    let program = machine.program;
    store_parameters(
        program,
        machine.active_frame_mut(),
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
    let Some(callee) = machine.program.functions.function_by_index(local_index) else {
        return Transfer::TailCall {
            function: *function,
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
        };
    };

    // collect argument values
    let argument_values = {
        let current_func = match machine
            .program
            .functions
            .function_by_id(machine.active_frame().function())
        {
            Some(function) => function,
            None => return Transfer::Error(Error::invalid_instruction()),
        };
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
    let function_id = machine.active_frame().function();
    let Some(function) = machine.program.functions.function_by_id(function_id) else {
        return Transfer::Error(Error::undefined_function(function_id));
    };

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
    let Some(frame_layout) = machine.program.frame_layout_by_id(function.frame_layout) else {
        return Transfer::Error(Error::invalid_instruction());
    };

    // discard stack allocations from the previous self call
    {
        let (stack_offset, frame_base) = {
            let frame = machine.active_frame_mut();
            (frame.stack_offset, frame.base_address())
        };
        let frame_byte_len = frame_layout.byte_len as usize;

        machine
            .interpreter
            .truncate_stack(stack_offset + frame_byte_len);
        let frame = machine.active_frame_mut();
        frame.retarget(function, entry, stack_offset, frame_byte_len, frame_base);
        frame.clear_values(frame_layout);
    }

    // load dispatch metadata after replacing frame bytes
    if let Err(error) = machine.load_active_frame() {
        return Transfer::Error(error);
    }

    // bind function parameters
    let program = machine.program;
    if let Err(error) = store_parameters(
        program,
        machine.active_frame_mut(),
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

/// Execute closure value tail call.
pub(crate) fn execute_tail_call_closure(
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
        slot,
        arguments,
    } = machine.side::<TailCallVirtual>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
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

/// Execute a dynamic tail call with a statically known receiver heap.
fn execute_tail_call_dynamic<const IS_SHARED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCallDynamic {
        receiver_offset,
        table_field,
        slot,
        arguments,
    } = machine.side::<TailCallDynamic>(instruction);

    // resolve dynamic callee
    let receiver_value = machine.load_word_at(*receiver_offset);
    let table_field = machine.projection(*table_field);
    let function_id =
        match resolve_dynamic_callee::<IS_SHARED>(machine, receiver_value, table_field, *slot) {
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

/// Execute dynamic tail call through a local heap receiver.
pub(crate) fn execute_tail_call_dynamic_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_dynamic::<false>(machine, instruction)
}

/// Execute dynamic tail call through a shared heap receiver.
pub(crate) fn execute_tail_call_dynamic_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_dynamic::<true>(machine, instruction)
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
    let callee_value = machine.load_word_at(*callee_offset);

    // resolve closure function and environment
    let (function_id, env) = match resolve_indirect_callee::<HAS_ENVIRONMENT>(machine, callee_value)
    {
        Ok(callee) => callee,
        Err(error) => return Transfer::Error(error),
    };
    let function = function_id.id;

    if let Err(error) =
        machine
            .program
            .functions
            .validate_signature(machine.tree(), function_id, *signature)
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
    let Some(callee) = machine.program.functions.function_by_index(local_index) else {
        return Transfer::TailCall {
            function,
            target,
            arguments: *arguments,
            env,
            moves: None,
        };
    };

    // collect argument values
    let caller = match machine.frame(machine.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = match machine.program.functions.function_by_id(caller.function()) {
        Some(function) => function,
        None => return Transfer::Error(Error::invalid_instruction()),
    };
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
