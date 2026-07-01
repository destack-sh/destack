use crate::diagnostic::Error;
use crate::machine::Activation;

use crate::Cell;
use destack_program::CellLayout;

use super::access;

/// Return the byte width of one function value word.
#[inline]
fn pointer_bytes(activation: &Activation<'_>) -> usize {
    activation.program.pointer_bytes() as usize
}

/// Return the code pointer byte offset in a function value.
#[inline]
fn code_word_offset() -> usize {
    0
}

/// Return the environment byte offset in a function value.
#[inline]
fn environment_offset(activation: &Activation<'_>) -> usize {
    pointer_bytes(activation)
}

/// Return the address of one function value word.
#[inline]
fn word_address(activation: &Activation<'_>, function_offset: u32, word_offset: usize) -> usize {
    activation.frame_pointer_at(function_offset).address() + word_offset
}

/// Store one function environment word into the function value.
fn store_environment(
    activation: &mut Activation<'_>,
    destination: u32,
    layout: CellLayout,
    source_offset: u32,
) {
    let environment = activation.load_cell_at(source_offset);
    let address = word_address(activation, destination, environment_offset(activation));

    access::store_scalar_by_layout_at_address(address, layout, environment);
}

/// Bind one function and environment into a frame-resident function value.
pub(crate) fn bind_function(
    activation: &mut Activation<'_>,
    destination: u32,
    function: Cell,
    environment: CellLayout,
    environment_offset: u32,
) {
    let address = word_address(activation, destination, code_word_offset());
    access::store_scalar_by_layout_at_address(address, CellLayout::FunctionPointer, function);

    store_environment(activation, destination, environment, environment_offset);
}

/// Decode one frame-resident function value into function and environment values.
pub(crate) fn decode_function(
    activation: &mut Activation<'_>,
    value_offset: u32,
) -> Result<(Cell, Cell), Error> {
    let function_address = word_address(activation, value_offset, code_word_offset());
    let function =
        access::load_scalar_by_layout_at_address(function_address, CellLayout::FunctionPointer);
    let function_id = function.as_function_pointer().function();

    let environment_layout = activation
        .machine
        .program
        .function_environment_layout(function_id)
        .ok_or(Error::invalid_instruction())?;
    let environment_address =
        word_address(activation, value_offset, environment_offset(activation));
    let environment =
        access::load_scalar_by_layout_at_address(environment_address, environment_layout);

    Ok((function, environment))
}
