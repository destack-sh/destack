use destack_program::vm::{
    ArgumentRange, Call, CallDynamic, CallTarget, CallVirtual, Cell, Drop, FunctionBind,
    FunctionCode, FunctionPointer, IndirectCall, IndirectTailCall, Instruction, Invoke,
    InvokeDynamic, InvokeIndirect, InvokeVirtual, MoveRange, Projection, TailCall, TailCallDynamic,
    TailCallVirtual,
};
use destack_program::{CellLayout, FunctionId, GlobalAddress, Program};

use super::frame::{
    FrameValue, load_arguments, load_moved_arguments, move_values, store_parameters,
};
use super::{Transfer, access, function};
use crate::diagnostic::Error;
use crate::machine::{Activation, Frame};

/// Load one lowered call table field from a receiver.
fn load_receiver_field<const IS_SHARED: bool>(
    activation: &mut Activation<'_>,
    receiver: Cell,
    field: Projection,
) -> Result<Cell, Error> {
    if IS_SHARED {
        return Ok(access::load_shared_heap_scalar::<8, false>(
            activation,
            receiver,
            field.byte_offset(),
        ));
    }

    Ok(access::load_heap_scalar::<8, false>(
        activation,
        receiver,
        field.byte_offset(),
    ))
}

/// Load one function target from an immutable dispatch table.
fn load_dispatch_slot(
    activation: &Activation<'_>,
    table_address: GlobalAddress,
    slot: u32,
) -> Result<FunctionId, Error> {
    // dispatch table slots are target pointers
    let pointer_bytes = activation.program.pointer_bytes() as usize;
    let byte_offset = slot as usize * pointer_bytes;
    let entry_address = table_address
        .add_bytes(byte_offset)
        .ok_or(Error::invalid_instruction())?;

    // static dispatch tables contain function addresses
    let function = load_function_pointer(activation, entry_address, pointer_bytes)?;
    let function = function.as_function_pointer().function();

    Ok(function)
}

/// Load one function address from static memory.
fn load_function_pointer(
    activation: &Activation<'_>,
    address: GlobalAddress,
    pointer_bytes: usize,
) -> Result<Cell, Error> {
    let address = activation.static_native_address(address, pointer_bytes)?;

    // read static bytes without assuming stronger alignment
    let raw = unsafe {
        match pointer_bytes {
            4 => u32::from_le((address as *const u32).read_unaligned()) as u64,
            8 => u64::from_le((address as *const u64).read_unaligned()),
            _ => return Err(Error::invalid_instruction()),
        }
    };

    Ok(CellLayout::FunctionPointer.decode(raw))
}

/// Resolve the callee for one virtual call.
fn resolve_virtual_callee<const IS_SHARED: bool>(
    activation: &mut Activation<'_>,
    receiver: Cell,
    table_field: Projection,
    slot: u32,
) -> Result<FunctionId, Error> {
    // load the vtable pointer from the receiver
    let vtable_value = load_receiver_field::<IS_SHARED>(activation, receiver, table_field)?;
    let vtable_pointer = vtable_value.as_global_address();

    load_dispatch_slot(activation, vtable_pointer, slot)
}

/// Resolve the callee for one dynamic call.
fn resolve_dynamic_callee(
    activation: &Activation<'_>,
    receiver_offset: u32,
    slot: u32,
) -> Result<FunctionId, Error> {
    // resolve the witness table carried directly by the dynamic value
    let table = activation
        .load_cell_at(receiver_offset + Cell::BYTE_LEN as u32)
        .as_dynamic_table();
    let entry = activation
        .program
        .dynamic_entry(table, slot)
        .ok_or_else(Error::invalid_instruction)?;

    entry
        .function_value()
        .ok_or_else(Error::invalid_instruction)
}

/// Resolve the callee for one indirect call.
fn resolve_indirect_callee<const HAS_ENVIRONMENT: bool>(
    activation: &mut Activation<'_>,
    callee_offset: u32,
) -> Result<(FunctionId, Option<Cell>), Error> {
    // function pointers are already the callee payload
    if !HAS_ENVIRONMENT {
        let callee = activation.load_cell_at(callee_offset);
        let function = callee.as_function_pointer().function();

        return Ok((function, None));
    }

    // function values carry a function pointer and environment pointer
    let (function, environment_value) = function::decode_function(activation, callee_offset)?;
    let function = function.as_function_pointer().function();

    Ok((function, Some(environment_value)))
}

/// Require one lowered call target for a function id.
#[inline]
fn require_call_target(
    activation: &Activation<'_>,
    function: FunctionId,
) -> Result<CallTarget, Error> {
    activation
        .program
        .vm_call_target(function)
        .ok_or(Error::undefined_function(function))
}

/// Load a function pointer.
pub(crate) fn execute_function_address(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // build function pointer value
    let dest = instruction.a;
    let function: FunctionId = instruction.b.into();
    let value = Cell::function_pointer(FunctionPointer::from(function));

    // store result
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Build a function value from one function and environment.
pub(crate) fn execute_function_bind(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let function: FunctionId = instruction.b.into();
    let environment_offset = instruction.c;
    let FunctionBind { environment } = *activation.side_record::<FunctionBind>(instruction.d);

    // bind the function pointer and environment into a function value
    let function = Cell::function_pointer(FunctionPointer::from(function));
    function::bind_function(
        activation,
        dest_offset,
        function,
        environment,
        environment_offset,
    );

    Ok(())
}

/// Load the function pointer from one function value.
pub(crate) fn execute_function_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination = instruction.a;
    let function_offset = instruction.b;

    // decode the function value
    let (function, _) = function::decode_function(activation, function_offset)?;

    // store result
    activation.store_cell_at(destination, function);

    Ok(())
}

/// Load the environment from one function value.
pub(crate) fn execute_function_environment(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination = instruction.a;
    let function_offset = instruction.b;

    // decode the function value
    let (_, environment) = function::decode_function(activation, function_offset)?;

    // store result
    activation.store_cell_at(destination, environment);

    Ok(())
}

/// Load the function environment pointer for the current frame.
pub(crate) fn execute_function_environment_current(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let destination = instruction.a;

    // load current frame environment
    let environment = activation
        .active_frame()
        .load_environment(activation.program, activation.frame_layout())?;
    let Some(environment) = environment else {
        return Err(Error::invalid_instruction());
    };

    // store result
    activation.store_cell_at(destination, environment);

    Ok(())
}

/// Return one local function for a call target.
#[inline]
fn local_function<'a>(program: &'a Program, target: CallTarget) -> Option<FunctionCode<'a>> {
    // only local callees can enter directly
    program.vm_function_by_index(target.local_index()?)
}

/// Enter one local callee without creating a call transfer.
#[inline]
fn enter_local_call(
    activation: &mut Activation<'_>,
    target: CallTarget,
    env: Option<Cell>,
    moves: Option<MoveRange>,
    resume_pc: usize,
) -> Option<Transfer> {
    // NOTE #Performance: avoid transfer plumbing for local calls
    let moves = moves?;

    // require one local callee before entering
    let program = activation.program;
    let callee = local_function(program, target)?;

    // reject stack overflow before mutating any live activation
    if activation.machine.frames.len() >= activation.machine.options.limits.max_stack_depth {
        return Some(Transfer::Error(Error::stack_overflow()));
    }

    // store the caller pc before allocating the callee
    {
        let caller = activation.active_frame_mut();
        caller.pc = resume_pc;
    }

    let layout = match activation
        .program
        .frame_layout_by_id(callee.function.frame_layout)
        .cloned()
    {
        Some(layout) => layout,
        None => return Some(Transfer::Error(Error::invalid_instruction())),
    };
    let (stack_offset, frame_base) = match activation.machine.allocate_frame(&layout) {
        Ok(frame) => frame,
        Err(error) => return Some(Transfer::Error(error.error)),
    };
    let mut new_frame = Frame::new(
        &callee,
        callee.function.entry,
        &layout,
        stack_offset,
        frame_base,
    );
    if let Err(error) = new_frame.store_environment(program, &layout, env) {
        return Some(Transfer::Error(error));
    }

    // bind parameters from the current caller frame
    let caller_index = activation.frame_index;
    let current_function = {
        let frame = activation.active_frame();
        match activation.program.vm_function_by_id(frame.function()) {
            Some(function) => function,
            None => return Some(Transfer::Error(Error::invalid_instruction())),
        }
    };
    let caller = match activation.frame(caller_index) {
        Ok(caller) => caller,
        Err(error) => return Some(Transfer::Error(error)),
    };

    if let Err(error) = move_values(caller, &mut new_frame, moves, current_function.move_pool) {
        return Some(Transfer::Error(error));
    }

    // push the callee frame and continue at its entry block
    activation.machine.frames.push(new_frame);
    let new_index = activation.machine.frames.len() - 1;
    if let Err(error) = activation.enter_frame(new_index) {
        return Some(Transfer::Error(error));
    }

    Some(Transfer::Enter)
}

/// Enter one call or return a call transfer.
fn enter_call(
    activation: &mut Activation<'_>,
    function_id: FunctionId,
    target: CallTarget,
    arguments: ArgumentRange,
    env: Option<Cell>,
    moves: Option<MoveRange>,
    resume_pc: usize,
    allow_direct: bool,
) -> Transfer {
    // enter local callees without bouncing through transfer handling
    if allow_direct
        && let Some(transfer) = enter_local_call(activation, target, env, moves, resume_pc)
    {
        return transfer;
    }

    // otherwise return the call to transfer handling
    Transfer::Call {
        function: function_id,
        target,
        arguments,
        env,
        moves,
        resume_pc,
    }
}

/// Execute direct function call.
pub(crate) fn execute_call(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let Call {
        function,
        target,
        arguments,
        moves,
    } = activation.side::<Call>(instruction);

    // load target function id
    let function_id = (*function).into();
    let moves = Some(*moves);

    // skip direct call when instruction limits are active
    let allow_direct = activation.machine.options.limits.max_instructions.is_none();

    enter_call(
        activation,
        function_id,
        *target,
        *arguments,
        None,
        moves,
        pc + 1,
        allow_direct,
    )
}

/// Execute one concrete value drop.
pub(crate) fn execute_drop(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    let Drop {
        function,
        target,
        value_offset,
    } = *activation.side::<Drop>(instruction);

    // pass the live world offset for one value
    let pointer = activation.frame_pointer_at(value_offset);
    let address = Cell::frame_pointer(pointer);

    Transfer::Drop {
        function: function.into(),
        target,
        address,
        resume_pc: pc + 1,
    }
}

/// Execute one direct invocation.
pub(crate) fn execute_invoke(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    let Invoke {
        function,
        target,
        arguments,
        normal_state,
        unwind_state,
    } = activation.side::<Invoke>(instruction);

    let function_id = (*function).into();

    Transfer::invoke(
        function_id,
        *target,
        *arguments,
        None,
        *normal_state,
        *unwind_state,
    )
}

/// Execute a class function call with a statically known receiver heap.
fn execute_call_virtual<const IS_SHARED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let record = *activation.side::<CallVirtual>(instruction);
    let CallVirtual {
        receiver_offset,
        table_field,
        slot,
        arguments,
    } = record;

    // resolve dynamic callee
    let receiver_value = activation.load_cell_at(receiver_offset);
    let table_field = activation.projection(table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(activation, receiver_value, table_field, slot) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = activation.machine.options.limits.max_instructions.is_none();

    enter_call(
        activation,
        function_id,
        target,
        arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Execute virtual call through a local receiver.
pub(crate) fn execute_call_virtual_local(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<false>(activation, instruction, pc)
}

/// Execute virtual call through a shared receiver.
pub(crate) fn execute_call_virtual_shared(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_call_virtual::<true>(activation, instruction, pc)
}

/// Execute a virtual invocation with a statically known receiver heap.
fn execute_invoke_virtual<const IS_SHARED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    let record = *activation.side::<InvokeVirtual>(instruction);
    let InvokeVirtual {
        receiver_offset,
        table_field,
        slot,
        arguments,
        normal_state,
        unwind_state,
    } = record;

    let receiver_value = activation.load_cell_at(receiver_offset);
    let table_field = activation.projection(table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(activation, receiver_value, table_field, slot) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::invoke(
        function_id,
        target,
        arguments,
        None,
        normal_state,
        unwind_state,
    )
}

/// Execute a virtual invocation through a local receiver.
pub(crate) fn execute_invoke_virtual_local(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_virtual::<false>(activation, instruction)
}

/// Execute a virtual invocation through a shared receiver.
pub(crate) fn execute_invoke_virtual_shared(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_invoke_virtual::<true>(activation, instruction)
}

/// Execute a dynamic function call.
pub(crate) fn execute_call_dynamic(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let record = *activation.side::<CallDynamic>(instruction);
    let CallDynamic {
        receiver_offset,
        slot,
        arguments,
    } = record;

    // resolve dynamic callee
    let function_id = match resolve_dynamic_callee(activation, receiver_offset, slot) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // skip direct call when instruction limits are active
    let allow_direct = activation.machine.options.limits.max_instructions.is_none();

    enter_call(
        activation,
        function_id,
        target,
        arguments,
        None,
        None,
        pc + 1,
        allow_direct,
    )
}

/// Execute a dynamic invocation.
pub(crate) fn execute_invoke_dynamic(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    let record = *activation.side::<InvokeDynamic>(instruction);
    let InvokeDynamic {
        receiver_offset,
        slot,
        arguments,
        normal_state,
        unwind_state,
    } = record;

    let function_id = match resolve_dynamic_callee(activation, receiver_offset, slot) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::invoke(
        function_id,
        target,
        arguments,
        None,
        normal_state,
        unwind_state,
    )
}

/// Execute an indirect call with a statically known callee.
fn execute_indirect_call<const HAS_ENVIRONMENT: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    // decode side records
    let record = *activation.side::<IndirectCall>(instruction);
    let IndirectCall {
        callee_offset,
        signature,
        arguments,
    } = record;

    // resolve function pointer and environment
    let (function_id, env) =
        match resolve_indirect_callee::<HAS_ENVIRONMENT>(activation, callee_offset) {
            Ok(callee) => callee,
            Err(error) => return Transfer::Error(error),
        };
    let function = function_id;

    let signature = activation.signature(signature);
    let parameters = activation.signature_parameters(signature);
    if let Err(error) =
        activation
            .program
            .check_function_signature_entry(function_id, signature, parameters)
    {
        return Transfer::Error(error.into());
    }

    // load the lowered call target
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // return control to transfer handling
    Transfer::Call {
        function,
        target,
        arguments,
        env,
        moves: None,
        resume_pc: pc + 1,
    }
}

/// Execute function pointer call.
pub(crate) fn execute_call_function_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_indirect_call::<false>(activation, instruction, pc)
}

/// Execute function value call.
pub(crate) fn execute_call_function(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    pc: usize,
) -> Transfer {
    execute_indirect_call::<true>(activation, instruction, pc)
}

/// Execute an indirect invocation with a statically known callee.
fn execute_indirect_invoke<const HAS_ENVIRONMENT: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    let record = *activation.side::<InvokeIndirect>(instruction);
    let InvokeIndirect {
        callee_offset,
        signature,
        arguments,
        normal_state,
        unwind_state,
    } = record;

    let (function_id, env) =
        match resolve_indirect_callee::<HAS_ENVIRONMENT>(activation, callee_offset) {
            Ok(callee) => callee,
            Err(error) => return Transfer::Error(error),
        };
    let signature = activation.signature(signature);
    let parameters = activation.signature_parameters(signature);
    if let Err(error) =
        activation
            .program
            .check_function_signature_entry(function_id, signature, parameters)
    {
        return Transfer::Error(error.into());
    }
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::invoke(
        function_id,
        target,
        arguments,
        env,
        normal_state,
        unwind_state,
    )
}

/// Execute one function pointer invocation.
pub(crate) fn execute_invoke_function_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_invoke::<false>(activation, instruction)
}

/// Execute one function value invocation.
pub(crate) fn execute_invoke_function(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_invoke::<true>(activation, instruction)
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    activation: &mut Activation<'_>,
    callee: &FunctionCode<'_>,
    argument_values: &[FrameValue],
    env: Option<Cell>,
) -> Result<(), Error> {
    // load callee frame layout
    let layout = activation
        .program
        .frame_layout_by_id(callee.function.frame_layout)
        .cloned()
        .ok_or(Error::invalid_instruction())?;

    // replace the current frame bytes in place
    let stack_offset = activation.active_frame_mut().stack_offset;
    activation.machine.truncate_stack(stack_offset);
    let (stack_offset, frame_base) = activation
        .machine
        .allocate_frame(&layout)
        .map_err(|error| error.error)?;
    let program = activation.program;
    {
        let frame = activation.active_frame_mut();
        frame.retarget(
            callee,
            callee.function.entry,
            stack_offset,
            layout.byte_len() as usize,
            frame_base,
        );
        frame.store_environment(program, &layout, env)?;
    }

    // bind dispatch tables for the retargeted frame
    let frame_index = activation.frame_index;
    activation.bind_frame(frame_index)?;

    // bind function parameters
    store_parameters(
        activation.active_frame_mut(),
        callee.argument_pool,
        callee.function.parameters,
        argument_values,
    )?;

    Ok(())
}

/// Execute tail call to function.
pub(crate) fn execute_tail_call(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TailCall {
        function,
        target,
        moves,
    } = activation.side::<TailCall>(instruction);

    // require the local callee
    let local_index = target.local_index();

    // return binding calls to transfer handling
    let Some(local_index) = local_index else {
        return Transfer::TailCall {
            function: (*function).into(),
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
        };
    };
    let program = activation.program;
    let Some(callee) = program.vm_function_by_index(local_index) else {
        return Transfer::TailCall {
            function: (*function).into(),
            target: *target,
            arguments: ArgumentRange::empty(),
            env: None,
            moves: Some(*moves),
        };
    };

    // collect argument values
    let argument_values = {
        let current_func = match program.vm_function_by_id(activation.active_frame().function()) {
            Some(function) => function,
            None => return Transfer::Error(Error::invalid_instruction()),
        };
        let caller = match activation.frame(activation.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_moved_arguments(caller, current_func.move_pool, *moves) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };

    // enter tail call
    if let Err(error) = enter_tail_call(activation, &callee, &argument_values, None) {
        return Transfer::Error(error);
    }

    Transfer::Enter
}

/// Execute self tail call by reusing the current frame.
pub(crate) fn execute_tail_call_self(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    let entry = instruction.a;
    let arguments = ArgumentRange {
        start: instruction.b,
        len: instruction.c,
    };

    // load current function entry block
    let function_id = activation.active_frame().function();
    let program = activation.program;
    let Some(function) = program.vm_function_by_id(function_id) else {
        return Transfer::Error(Error::undefined_function(function_id));
    };

    // collect argument values before clearing the frame
    let args = {
        let caller = match activation.frame(activation.frame_index) {
            Ok(frame) => frame,
            Err(error) => return Transfer::Error(error),
        };

        match load_arguments(caller, function.argument_pool, arguments) {
            Ok(arguments) => arguments,
            Err(error) => return Transfer::Error(error),
        }
    };
    let Some(frame_layout) = program
        .frame_layout_by_id(function.function.frame_layout)
        .cloned()
    else {
        return Transfer::Error(Error::invalid_instruction());
    };

    // discard stack allocations from the previous self call
    {
        let (stack_offset, frame_base) = {
            let frame = activation.active_frame_mut();
            (frame.stack_offset, frame.base_address())
        };
        let frame_byte_len = frame_layout.byte_len() as usize;

        activation
            .machine
            .truncate_stack(stack_offset + frame_byte_len);
        let frame = activation.active_frame_mut();
        frame.retarget(&function, entry, stack_offset, frame_byte_len, frame_base);
        frame.clear_values(program, &frame_layout);
    }

    // bind dispatch tables after replacing frame bytes
    let frame_index = activation.frame_index;
    if let Err(error) = activation.bind_frame(frame_index) {
        return Transfer::Error(error);
    }

    // bind function parameters
    if let Err(error) = store_parameters(
        activation.active_frame_mut(),
        function.argument_pool,
        function.function.parameters,
        &args,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Enter
}

/// Execute indirect tail call.
pub(crate) fn execute_tail_call_function_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_tail_call::<false>(activation, instruction)
}

/// Execute function value tail call.
pub(crate) fn execute_tail_call_function(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_indirect_tail_call::<true>(activation, instruction)
}

/// Execute a virtual tail call with a statically known receiver heap.
fn execute_tail_call_virtual<const IS_SHARED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let record = *activation.side::<TailCallVirtual>(instruction);
    let TailCallVirtual {
        receiver_offset,
        table_field,
        slot,
        arguments,
    } = record;

    // resolve dynamic callee
    let receiver_value = activation.load_cell_at(receiver_offset);
    let table_field = activation.projection(table_field);
    let function_id =
        match resolve_virtual_callee::<IS_SHARED>(activation, receiver_value, table_field, slot) {
            Ok(function_id) => function_id,
            Err(error) => return Transfer::Error(error),
        };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::TailCall {
        function: function_id,
        target,
        arguments,
        env: None,
        moves: None,
    }
}

/// Execute virtual tail call through a local receiver.
pub(crate) fn execute_tail_call_virtual_local(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_virtual::<false>(activation, instruction)
}

/// Execute virtual tail call through a shared receiver.
pub(crate) fn execute_tail_call_virtual_shared(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tail_call_virtual::<true>(activation, instruction)
}

/// Execute a dynamic tail call.
pub(crate) fn execute_tail_call_dynamic(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let record = *activation.side::<TailCallDynamic>(instruction);
    let TailCallDynamic {
        receiver_offset,
        slot,
        arguments,
    } = record;

    // resolve dynamic callee
    let function_id = match resolve_dynamic_callee(activation, receiver_offset, slot) {
        Ok(function_id) => function_id,
        Err(error) => return Transfer::Error(error),
    };
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::TailCall {
        function: function_id,
        target,
        arguments,
        env: None,
        moves: None,
    }
}

/// Execute an indirect tail call with a statically known callee.
fn execute_indirect_tail_call<const HAS_ENVIRONMENT: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let record = *activation.side::<IndirectTailCall>(instruction);
    let IndirectTailCall {
        callee_offset,
        signature,
        arguments,
    } = record;

    // resolve function pointer and environment
    let (function_id, env) =
        match resolve_indirect_callee::<HAS_ENVIRONMENT>(activation, callee_offset) {
            Ok(callee) => callee,
            Err(error) => return Transfer::Error(error),
        };
    let function = function_id;

    let signature = activation.signature(signature);
    let parameters = activation.signature_parameters(signature);
    if let Err(error) =
        activation
            .program
            .check_function_signature_entry(function_id, signature, parameters)
    {
        return Transfer::Error(error.into());
    }

    // load the lowered call target
    let target = match require_call_target(activation, function_id) {
        Ok(target) => target,
        Err(error) => return Transfer::Error(error),
    };

    // return binding calls to transfer handling
    let local_index = target.local_index();
    let Some(local_index) = local_index else {
        return Transfer::TailCall {
            function,
            target,
            arguments,
            env,
            moves: None,
        };
    };
    let program = activation.program;
    let Some(callee) = program.vm_function_by_index(local_index) else {
        return Transfer::TailCall {
            function,
            target,
            arguments,
            env,
            moves: None,
        };
    };

    // collect argument values
    let caller = match activation.frame(activation.frame_index) {
        Ok(frame) => frame,
        Err(error) => return Transfer::Error(error),
    };
    let caller_function = match program.vm_function_by_id(caller.function()) {
        Some(function) => function,
        None => return Transfer::Error(Error::invalid_instruction()),
    };
    let argument_values = match load_arguments(caller, caller_function.argument_pool, arguments) {
        Ok(arguments) => arguments,
        Err(error) => return Transfer::Error(error),
    };

    // enter tail call
    if let Err(error) = enter_tail_call(activation, &callee, &argument_values, env) {
        return Transfer::Error(error);
    }

    Transfer::Enter
}
