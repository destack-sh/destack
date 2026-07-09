use destack_program::vm::{FunctionCode, Instruction};

use crate::diagnostic::Error;
use crate::machine::Activation;

/// Execute one explicit profile counter increment.
#[inline(always)]
pub(crate) fn execute_profile_increment(
    activation: &mut Activation<'_>,
    function: FunctionCode<'_>,
    block: u32,
    pc: usize,
) -> Result<(), Error> {
    activation.record_profile_counter_at(function, block, pc)
}

/// Execute one explicit profile sample.
#[inline(always)]
pub(crate) fn execute_profile_sample(
    activation: &mut Activation<'_>,
    function: FunctionCode<'_>,
    block: u32,
    pc: usize,
    instruction: &Instruction,
) -> Result<(), Error> {
    let value = activation.load_cell_at(instruction.a);

    activation.record_profile_sample_at(function, block, pc, value.bits())
}
