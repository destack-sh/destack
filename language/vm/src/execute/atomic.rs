use crate::interpreter::DispatchState;
use crate::program::{
    AtomicCompareExchange, AtomicFence, AtomicLoad, AtomicRmw, AtomicStore, Instruction, Transfer,
};
use destack_mir as mir;

/// Execute atomic load.
pub(crate) fn execute_atomic_load(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicLoad {
        dest,
        pointer,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicLoad>();

    // execute the load
    let pointer = state.get(*pointer);
    let value = match state.atomic_load_value(
        pointer,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute atomic store.
pub(crate) fn execute_atomic_store(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicStore {
        pointer,
        value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicStore>();

    // execute the store
    let pointer = state.get(*pointer);
    let value = state.get(*value);
    if let Err(error) = state.atomic_store_value(
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}

/// Execute atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicCompareExchange {
        dest,
        pointer,
        expected,
        new_value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicCompareExchange>();

    // execute the compare exchange
    let pointer = state.get(*pointer);
    let expected = state.get(*expected);
    let new_value = state.get(*new_value);
    if let Err(error) = state.atomic_compare_exchange_value(
        *dest,
        pointer,
        expected,
        new_value,
        *raw_pointee,
        false,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}

/// Execute atomic read modify write.
pub(crate) fn execute_atomic_rmw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let AtomicRmw {
        dest,
        operator,
        pointer,
        value,
        raw_pointee,
        ..
    } = instruction.payload_as::<AtomicRmw>();

    // execute the read modify write
    let pointer = state.get(*pointer);
    let value = state.get(*value);
    let result = match state.atomic_rmw_value(
        *operator,
        pointer,
        value,
        *raw_pointee,
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error.error),
    };

    // store the result
    state.set_word(*dest, result);

    Transfer::Continue
}

/// Execute atomic fence.
pub(crate) fn execute_atomic_fence(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction.payload_as::<AtomicFence>();

    // execute the fence
    if let Err(error) = state.atomic_fence(
        mir::MemoryOrdering::SequentiallyConsistent,
        mir::AtomicScope::Device,
        mir::MemoryScope::Device,
        mir::MemorySemantics::default(),
    ) {
        return Transfer::Error(error.error);
    }

    Transfer::Continue
}
