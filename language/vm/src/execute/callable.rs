use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::{HeapReference, Word};
use destack_mir as mir;

use crate::program::{CallableEnvironment, CallableObjectLayout, WordLayout};

use super::access;

/// Decode one callable object into function and environment values.
fn decode_callable_object(
    machine: &mut Machine<'_, '_>,
    reference: HeapReference,
) -> Result<(Word, Word), Error> {
    let layout = machine.program.callable_object_layout();
    let base_address = machine.heap_address(reference, 0);

    // split the two pointer fields
    let function_address = base_address + layout.function_offset;
    let environment_address = base_address + layout.environment_offset;
    let function =
        access::load_scalar_by_layout_at_address(function_address, WordLayout::FunctionPointer)
            .as_function_pointer();
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function = Word::function_pointer(function);

    // decode the environment through lowered callable metadata
    let environment_layout = machine
        .program
        .functions
        .environment_layout(machine.tree(), function_id)
        .ok_or(Error::invalid_instruction())?;
    let environment_value =
        access::load_scalar_by_layout_at_address(environment_address, environment_layout);

    Ok((function, environment_value))
}

/// Encode one word callable environment into pointer-sized bits.
fn encode_callable_word_environment(
    machine: &mut Machine<'_, '_>,
    layout: WordLayout,
    environment_offset: u32,
) -> u64 {
    let environment = machine.load_word_at(environment_offset);

    layout.encode(environment)
}

/// Encode one frame callable environment into pointer-sized bits.
fn encode_callable_address_environment(
    machine: &mut Machine<'_, '_>,
    layout: mir::LayoutId,
    byte_len: usize,
    environment_offset: u32,
) -> Result<u64, Error> {
    let environment_reference = machine.with_frame_bytes_at(
        environment_offset,
        byte_len,
        |machine, environment_bytes| machine.allocate_heap_layout_bytes(layout, environment_bytes),
    )?;

    Ok(environment_reference.bits() as u64)
}

/// Encode one callable environment into pointer-sized bits.
fn encode_callable_environment(
    machine: &mut Machine<'_, '_>,
    environment: CallableEnvironment,
    environment_offset: u32,
) -> Result<u64, Error> {
    match environment {
        CallableEnvironment::Word { layout } => Ok(encode_callable_word_environment(
            machine,
            layout,
            environment_offset,
        )),
        CallableEnvironment::Frame { layout, byte_len } => {
            encode_callable_address_environment(machine, layout, byte_len, environment_offset)
        }
    }
}

/// Bind one function and encoded environment into a callable value.
fn bind_callable_object(
    machine: &mut Machine<'_, '_>,
    callable_layout: mir::LayoutId,
    object_layout: CallableObjectLayout,
    function: Word,
    environment_bits: u64,
) -> Result<Word, Error> {
    let function = function.as_function_pointer();
    let function_bytes = (function.bits() as u64).to_le_bytes();
    let environment_bytes = environment_bits.to_le_bytes();
    let mut bytes = [0u8; Word::BYTE_LEN * 2];
    let bytes = &mut bytes[..object_layout.byte_len];

    // place the two pointer sized fields
    let pointer_bytes = object_layout.alignment;
    let function_end = object_layout.function_offset + pointer_bytes;
    let environment_end = object_layout.environment_offset + pointer_bytes;
    bytes[object_layout.function_offset..function_end]
        .copy_from_slice(&function_bytes[..pointer_bytes]);
    bytes[object_layout.environment_offset..environment_end]
        .copy_from_slice(&environment_bytes[..pointer_bytes]);

    // allocate the callable object
    let reference = machine.allocate_heap_layout_bytes(callable_layout, bytes)?;

    Ok(Word::heap_reference(reference))
}

/// Bind one function and environment into a callable value.
pub(crate) fn bind_callable(
    machine: &mut Machine<'_, '_>,
    callable_layout: mir::LayoutId,
    object_layout: CallableObjectLayout,
    function: Word,
    environment: CallableEnvironment,
    environment_offset: u32,
) -> Result<Word, Error> {
    let environment_bits = encode_callable_environment(machine, environment, environment_offset)?;

    bind_callable_object(
        machine,
        callable_layout,
        object_layout,
        function,
        environment_bits,
    )
}

/// Decode one callable value into function and environment values.
pub(crate) fn decode_callable(
    machine: &mut Machine<'_, '_>,
    value: Word,
) -> Result<(Word, Word), Error> {
    let reference = value.as_heap_reference();
    if machine.is_heap_live(reference) {
        return decode_callable_object(machine, reference);
    }

    Err(Error::type_mismatch(
        "callable object",
        format!("{value:?}"),
    ))
}
