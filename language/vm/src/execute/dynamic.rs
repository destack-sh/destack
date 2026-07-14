use destack_program::vm::{Cell, Instruction};

use crate::diagnostic::Error;
use crate::machine::Activation;

/// Bind an erased payload to one dynamic table.
pub(crate) fn execute_dynamic_bind(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let payload = activation.load_cell_at(instruction.b);
    let table = Cell::dynamic_table(instruction.c.into());

    activation.store_cell_at(instruction.a, payload);
    activation.store_cell_at(instruction.a + Cell::BYTE_LEN as u32, table);

    Ok(())
}

/// Load the payload from one dynamic value.
pub(crate) fn execute_dynamic_payload(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let payload = activation.load_cell_at(instruction.b);
    activation.store_cell_at(instruction.a, payload);

    Ok(())
}

/// Load the concrete type from one dynamic value.
pub(crate) fn execute_dynamic_type(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // resolve concrete identity through the durable witness table
    let table = activation
        .load_cell_at(instruction.b + Cell::BYTE_LEN as u32)
        .as_dynamic_table();
    let table = activation
        .program
        .dynamic_table(table)
        .ok_or_else(Error::invalid_instruction)?;

    activation.store_cell_at(instruction.a, Cell::uint32(table.concrete.0));

    Ok(())
}

/// Release the boxed payload root from one dynamic value.
pub(crate) fn execute_drop_dynamic(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    activation.store_cell_at(instruction.a, Cell::ZERO);
    activation.store_cell_at(instruction.a + Cell::BYTE_LEN as u32, Cell::ZERO);

    Ok(())
}
