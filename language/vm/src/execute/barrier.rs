use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, Transfer};
use destack_mir as mir;

/// Execute a local heap barrier write.
pub(crate) fn execute_barrier_write_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let object = mir::Value::new(instruction.a);
    let offset = mir::Value::new(instruction.b);
    let byte_len = mir::Value::new(instruction.c);

    // load barrier range
    let object = machine.get(object);
    let offset = machine.get(offset).as_uint() as usize;
    let byte_len = machine.get(byte_len).as_uint() as usize;

    // publish to the local collector
    let result = machine
        .heap_mut()
        .write_barrier(object.as_heap_reference(), offset, byte_len);

    // report invalid heap ranges
    if let Err(error) = result {
        return Transfer::Error(Error::from(error));
    }

    Transfer::Continue
}

/// Execute a shared heap barrier write.
pub(crate) fn execute_barrier_write_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let object = mir::Value::new(instruction.a);
    let offset = mir::Value::new(instruction.b);
    let byte_len = mir::Value::new(instruction.c);

    // load barrier range
    let object = machine.get(object);
    let offset = machine.get(offset).as_uint() as usize;
    let byte_len = machine.get(byte_len).as_uint() as usize;

    // publish to the shared collector
    let result =
        machine
            .shared()
            .write_barrier(object.as_shared_heap_reference(), offset, byte_len);

    // report invalid heap ranges
    if let Err(error) = result {
        return Transfer::Error(Error::from(error));
    }

    Transfer::Continue
}
