use super::prelude::*;

/// Execute cast opcode.
pub(crate) fn execute_cast(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Cast {
        dest,
        op,
        arg,
        to_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute cast
    let cast_type = type_id(*to_type);
    let result = match operator::execute_cast(state.tree(), *op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute select opcode.
pub(crate) fn execute_select(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Select {
        dest,
        condition,
        then_value,
        else_value,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load condition and select result
    let cond = state.get(*condition).as_bool();
    let result = if cond {
        state.get(*then_value)
    } else {
        state.get(*else_value)
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}
