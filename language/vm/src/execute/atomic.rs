use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    AtomicCompareExchange, AtomicFence, AtomicLoad, AtomicRmw, AtomicStore, Instruction,
};

macro_rules! atomic_rmw_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            let AtomicRmw {
                dest_offset,
                pointer_offset,
                value_offset,
                layout,
                byte_len,
                ordering,
                scope,
                memory_scope,
                semantics,
            } = machine.side::<AtomicRmw>(instruction);

            // execute the read modify write
            let pointer = machine.get_word_at(*pointer_offset);
            let value = machine.get_word_at(*value_offset);
            let result = match machine.$operation(
                pointer,
                *layout,
                *byte_len,
                value,
                *ordering,
                *scope,
                *memory_scope,
                *semantics,
            ) {
                Ok(result) => result,
                Err(error) => return Err(error.error),
            };

            // store the result
            machine.set_word_at(*dest_offset, result);

            Ok(())
        }
    };
}

/// Execute atomic load.
pub(crate) fn execute_atomic_load(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let AtomicLoad {
        dest_offset,
        pointer_offset,
        layout,
        byte_len,
        ordering,
        scope,
        memory_scope,
        semantics,
    } = machine.side::<AtomicLoad>(instruction);

    // execute the load
    let pointer = machine.get_word_at(*pointer_offset);
    let value = match machine.atomic_load_value(
        pointer,
        *layout,
        *byte_len,
        *ordering,
        *scope,
        *memory_scope,
        *semantics,
    ) {
        Ok(value) => value,
        Err(error) => return Err(error.error),
    };

    // store the result
    machine.set_word_at(*dest_offset, value);

    Ok(())
}

/// Execute atomic store.
pub(crate) fn execute_atomic_store(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let AtomicStore {
        pointer_offset,
        value_offset,
        layout,
        byte_len,
        ordering,
        scope,
        memory_scope,
        semantics,
    } = machine.side::<AtomicStore>(instruction);

    // execute the store
    let pointer = machine.get_word_at(*pointer_offset);
    let value = machine.get_word_at(*value_offset);
    if let Err(error) = machine.atomic_store_value(
        pointer,
        value,
        *layout,
        *byte_len,
        *ordering,
        *scope,
        *memory_scope,
        *semantics,
    ) {
        return Err(error.error);
    }

    Ok(())
}

/// Execute atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode fixed fields
    let AtomicCompareExchange {
        dest,
        pointer_offset,
        expected_offset,
        new_value_offset,
        layout,
        byte_len,
        is_weak,
        ordering,
        scope,
        memory_scope,
        semantics,
    } = machine.side::<AtomicCompareExchange>(instruction);

    // execute the compare exchange
    let pointer = machine.get_word_at(*pointer_offset);
    let expected = machine.get_word_at(*expected_offset);
    let new_value = machine.get_word_at(*new_value_offset);
    if let Err(error) = machine.atomic_compare_exchange_value(
        *dest,
        pointer,
        expected,
        new_value,
        *layout,
        *byte_len,
        *is_weak,
        *ordering,
        *scope,
        *memory_scope,
        *semantics,
    ) {
        return Err(error.error);
    }

    Ok(())
}

atomic_rmw_executor!(
    execute_atomic_exchange,
    atomic_exchange,
    "Execute atomic exchange."
);
atomic_rmw_executor!(execute_atomic_add, atomic_fetch_add, "Execute atomic add.");
atomic_rmw_executor!(
    execute_atomic_sub,
    atomic_fetch_sub,
    "Execute atomic subtract."
);
atomic_rmw_executor!(execute_atomic_and, atomic_fetch_and, "Execute atomic and.");
atomic_rmw_executor!(execute_atomic_or, atomic_fetch_or, "Execute atomic or.");
atomic_rmw_executor!(execute_atomic_xor, atomic_fetch_xor, "Execute atomic xor.");
atomic_rmw_executor!(
    execute_atomic_min,
    atomic_fetch_min,
    "Execute atomic signed minimum."
);
atomic_rmw_executor!(
    execute_atomic_max,
    atomic_fetch_max,
    "Execute atomic signed maximum."
);
atomic_rmw_executor!(
    execute_atomic_umin,
    atomic_fetch_umin,
    "Execute atomic unsigned minimum."
);
atomic_rmw_executor!(
    execute_atomic_umax,
    atomic_fetch_umax,
    "Execute atomic unsigned maximum."
);
atomic_rmw_executor!(
    execute_atomic_fadd,
    atomic_fetch_fadd,
    "Execute atomic float add."
);
atomic_rmw_executor!(
    execute_atomic_fmin,
    atomic_fetch_fmin,
    "Execute atomic float minimum."
);
atomic_rmw_executor!(
    execute_atomic_fmax,
    atomic_fetch_fmax,
    "Execute atomic float maximum."
);

/// Execute atomic fence.
pub(crate) fn execute_atomic_fence(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let AtomicFence {
        ordering,
        scope,
        memory_scope,
        semantics,
    } = machine.side::<AtomicFence>(instruction);

    // execute the fence
    if let Err(error) = machine.atomic_fence(*ordering, *scope, *memory_scope, *semantics) {
        return Err(error.error);
    }

    Ok(())
}
