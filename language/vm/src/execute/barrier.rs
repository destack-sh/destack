use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::Instruction;

/// Execute a local heap barrier write.
pub(crate) fn execute_barrier_write_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let object = instruction.a;
    let offset = instruction.b;
    let byte_len = instruction.c;

    // load barrier range
    let object = machine.get_word_at(object);
    let offset = machine.get_word_at(offset).as_uint() as usize;
    let byte_len = machine.get_word_at(byte_len).as_uint() as usize;

    // publish to the local collector
    let result = machine
        .heap_mut()
        .write_barrier(object.as_heap_reference(), offset, byte_len);

    // report invalid heap ranges
    if let Err(error) = result {
        return Err(Error::from(error));
    }

    Ok(())
}

/// Execute a shared heap barrier write.
pub(crate) fn execute_barrier_write_shared_heap(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let object = instruction.a;
    let offset = instruction.b;
    let byte_len = instruction.c;

    // load barrier range
    let object = machine.get_word_at(object);
    let offset = machine.get_word_at(offset).as_uint() as usize;
    let byte_len = machine.get_word_at(byte_len).as_uint() as usize;

    // publish to the shared collector
    let result =
        machine
            .shared()
            .write_barrier(object.as_shared_heap_reference(), offset, byte_len);

    // report invalid heap ranges
    if let Err(error) = result {
        return Err(Error::from(error));
    }

    Ok(())
}
