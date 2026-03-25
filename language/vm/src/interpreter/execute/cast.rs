use super::*;

/// Handle cast operation.
pub(crate) fn handle_cast(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let InstructionData::Cast {
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
    let result = match operator::execute_cast(state.tree(), *op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle select operation.
pub(crate) fn handle_select(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let InstructionData::Select {
        dest,
        condition,
        then_value,
        else_value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load condition and select result
    let cond = state.get(*condition).as_bool().unwrap_or(false);
    let result = if cond {
        state.get(*then_value)
    } else {
        state.get(*else_value)
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
