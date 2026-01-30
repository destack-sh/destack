use super::*;

/// Handle intrinsic call.
pub(crate) fn handle_intrinsic(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Intrinsic {
        dest,
        intrinsic,
        arguments,
        ordering,
        scope,
        memory_scope,
        semantics,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve arguments
    let args = collect_values(state, *arguments);

    // execute intrinsic
    let heap_ptr = state.heap_ptr();
    let heap = unsafe { &mut *heap_ptr };
    let interpreter = &mut *state.interpreter;

    match interpreter.execute_intrinsic_resolved(
        heap,
        *intrinsic,
        args.as_slice(),
        *ordering,
        *scope,
        *memory_scope,
        *semantics,
    ) {
        // store result and continue
        Ok(result) => {
            if !is_invalid_value(*dest) {
                state.set(*dest, result);
            }
            next!(state, block, pc)
        }
        // return runtime error
        Err(e) => ControlFlow::Error(e.error),
    }
}
