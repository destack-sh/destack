use destack_heap::HeapReference;

use crate::Cell;
use crate::diagnostic::Error;
use crate::machine::Activation;
use destack_mir as mir;

use crate::program::{CellLayout, ClosureEnvironment, ClosureObjectLayout};

use super::access;

/// Decode one closure object into function and environment values.
fn decode_closure_object(
    activation: &mut Activation<'_>,
    reference: HeapReference,
) -> Result<(Cell, Cell), Error> {
    let layout = activation.machine.program.closure_object_layout();
    let base_address = activation.heap_address(reference, 0);

    // split the two pointer fields
    let function_address = base_address + layout.function_offset;
    let environment_address = base_address + layout.environment_offset;
    let function =
        access::load_scalar_by_layout_at_address(function_address, CellLayout::FunctionPointer)
            .as_function_pointer();
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function = Cell::function_pointer(function);

    // decode the environment through lowered function metadata
    let environment_layout = activation
        .machine
        .program
        .functions
        .environment_layout(activation.machine.tree(), function_id)
        .ok_or(Error::invalid_instruction())?;
    let environment_value =
        access::load_scalar_by_layout_at_address(environment_address, environment_layout);

    Ok((function, environment_value))
}

/// Encode one cell closure environment into pointer-sized bits.
fn encode_closure_cell_environment(
    activation: &mut Activation<'_>,
    layout: CellLayout,
    environment_offset: u32,
) -> u64 {
    let environment = activation.load_cell_at(environment_offset);

    layout.encode(environment)
}

/// Encode one frame closure environment into pointer-sized bits.
fn encode_closure_address_environment(
    activation: &mut Activation<'_>,
    layout: mir::LayoutId,
    byte_len: usize,
    environment_offset: u32,
) -> Result<u64, Error> {
    let environment_reference = activation.with_frame_bytes_at(
        environment_offset,
        byte_len,
        |activation, environment_bytes| {
            activation.allocate_heap_layout_bytes(layout, environment_bytes)
        },
    )?;

    Ok(environment_reference.bits() as u64)
}

/// Encode one closure environment into pointer-sized bits.
fn encode_closure_environment(
    activation: &mut Activation<'_>,
    environment: ClosureEnvironment,
    environment_offset: u32,
) -> Result<u64, Error> {
    match environment {
        ClosureEnvironment::Cell { layout } => Ok(encode_closure_cell_environment(
            activation,
            layout,
            environment_offset,
        )),
        ClosureEnvironment::Frame { layout, byte_len } => {
            encode_closure_address_environment(activation, layout, byte_len, environment_offset)
        }
    }
}

/// Bind one function and encoded environment into a closure value.
fn bind_closure_object(
    activation: &mut Activation<'_>,
    closure_layout: mir::LayoutId,
    object_layout: ClosureObjectLayout,
    function: Cell,
    environment_bits: u64,
) -> Result<Cell, Error> {
    let function = function.as_function_pointer();
    let function_bytes = (function.bits() as u64).to_le_bytes();
    let environment_bytes = environment_bits.to_le_bytes();
    let mut bytes = [0u8; Cell::BYTE_LEN * 2];
    let bytes = &mut bytes[..object_layout.byte_len];

    // place the two pointer sized fields
    let pointer_bytes = object_layout.alignment;
    let function_end = object_layout.function_offset + pointer_bytes;
    let environment_end = object_layout.environment_offset + pointer_bytes;
    bytes[object_layout.function_offset..function_end]
        .copy_from_slice(&function_bytes[..pointer_bytes]);
    bytes[object_layout.environment_offset..environment_end]
        .copy_from_slice(&environment_bytes[..pointer_bytes]);

    // allocate the closure object
    let reference = activation.allocate_heap_layout_bytes(closure_layout, bytes)?;

    Ok(Cell::heap_reference(reference))
}

/// Bind one function and environment into a closure value.
pub(crate) fn bind_closure(
    activation: &mut Activation<'_>,
    closure_layout: mir::LayoutId,
    object_layout: ClosureObjectLayout,
    function: Cell,
    environment: ClosureEnvironment,
    environment_offset: u32,
) -> Result<Cell, Error> {
    let environment_bits = encode_closure_environment(activation, environment, environment_offset)?;

    bind_closure_object(
        activation,
        closure_layout,
        object_layout,
        function,
        environment_bits,
    )
}

/// Decode one closure value into function and environment values.
pub(crate) fn decode_closure(
    activation: &mut Activation<'_>,
    value: Cell,
) -> Result<(Cell, Cell), Error> {
    let reference = value.as_heap_reference();
    if activation.is_heap_live(reference) {
        return decode_closure_object(activation, reference);
    }

    Err(Error::type_mismatch("closure object", format!("{value:?}")))
}
