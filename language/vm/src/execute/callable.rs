use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::{FunctionPointer, HeapReference, Word};
use destack_mir as mir;

use crate::program::{callable_object_layout, decode_word_bits, encode_word_bytes};

use super::access::load_scalar_bits;

/// Return the environment type for one bound function.
fn callable_environment_type(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let function = tree.get(function_id);
    let environment = function.environment.ok_or(Error::InvalidInstruction)?;

    environment
        .ty()
        .ok_or_else(|| Error::MissingRepresentation {
            context: "callable environment type".to_string(),
        })
}

/// Decode one callable object into function and environment values.
fn decode_callable_object(
    machine: &mut Machine<'_, '_>,
    reference: HeapReference,
) -> Result<(Word, Word), Error> {
    let layout = callable_object_layout(machine.tree().pointer_bytes() as usize);
    let pointer_bytes = machine.tree().pointer_bytes() as usize;
    let base_address = machine.heap().heap_base_address() + reference.offset();

    // split the two pointer fields
    let function_address = base_address + layout.function_offset;
    let environment_address = base_address + layout.environment_offset;
    let function_raw = load_scalar_bits(function_address, pointer_bytes);
    let environment_raw = load_scalar_bits(environment_address, pointer_bytes);

    let function = FunctionPointer::from_bits(function_raw as usize);
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function = Word::function_pointer(function);

    // decode small environments directly, otherwise decode the heap reference
    let environment_type = callable_environment_type(machine.tree(), function_id)?;
    let environment_value = if machine.layout(environment_type)?.is_word() {
        decode_word_bits(
            machine.tree(),
            environment_type,
            environment_raw,
            pointer_bytes,
        )?
    } else {
        let environment_reference = HeapReference::from_bits(environment_raw as usize);

        Word::heap_reference(environment_reference)
    };

    Ok((function, environment_value))
}

/// Encode one callable environment value into pointer-sized bits.
fn encode_callable_environment(
    machine: &mut Machine<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    environment_value: mir::Value,
) -> Result<u64, Error> {
    let environment_type = callable_environment_type(machine.tree(), function_id)?;
    if machine.layout(environment_type)?.is_word() {
        let environment = encode_word_bytes(
            machine.tree(),
            environment_type,
            machine.get(environment_value),
        )?;
        let pointer_bytes = machine.tree().pointer_bytes() as usize;
        let mut bytes = [0u8; Word::BYTE_LEN];

        bytes[..pointer_bytes].copy_from_slice(&environment.as_slice()[..pointer_bytes]);

        return Ok(u64::from_le_bytes(bytes));
    }

    // non-word environments are copied into one heap allocation
    let environment_layout_id = machine
        .program
        .layout_id_for_type(environment_type)
        .ok_or(Error::InvalidInstruction)?;
    let environment_bytes = machine.value_bytes(environment_value)?.to_vec();
    let environment_reference =
        machine.allocate_heap_layout_bytes(environment_layout_id, &environment_bytes)?;

    Ok(environment_reference.bits() as u64)
}

/// Bind one function and environment into a callable value.
pub(crate) fn bind_callable(
    machine: &mut Machine<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    function: Word,
    environment_value: mir::Value,
) -> Result<Word, Error> {
    let layout = callable_object_layout(machine.tree().pointer_bytes() as usize);
    let function = function.as_function_pointer();
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function_bytes = (function.bits() as u64).to_le_bytes();
    let environment_bytes =
        encode_callable_environment(machine, function_id, environment_value)?.to_le_bytes();
    let mut bytes = [0u8; Word::BYTE_LEN * 2];
    let bytes = &mut bytes[..layout.byte_len];

    // place the two pointer sized fields
    let pointer_bytes = machine.tree().pointer_bytes() as usize;
    let function_end = layout.function_offset + pointer_bytes;
    let environment_end = layout.environment_offset + pointer_bytes;
    bytes[layout.function_offset..function_end].copy_from_slice(&function_bytes[..pointer_bytes]);
    bytes[layout.environment_offset..environment_end]
        .copy_from_slice(&environment_bytes[..pointer_bytes]);

    // allocate the callable object
    let layout_id = machine
        .program
        .layout_id_for_type(ty)
        .ok_or(Error::InvalidInstruction)?;
    let reference = machine.allocate_heap_layout_bytes(layout_id, bytes)?;

    Ok(Word::heap_reference(reference))
}

/// Decode one callable value into function and environment values.
pub(crate) fn decode_callable(
    machine: &mut Machine<'_, '_>,
    value: Word,
) -> Result<(Word, Word), Error> {
    let reference = value.as_heap_reference();
    if machine.heap().is_heap_live(reference) {
        return decode_callable_object(machine, reference);
    }

    Err(Error::TypeMismatch {
        expected: "callable object".to_string(),
        actual: format!("{value:?}"),
    })
}
