use destack_bytecode::{Instruction, Opcode};
use destack_program::{CounterId, SamplerId};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one explicit profile operation.
    pub(crate) fn execute_profile(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let frame = self.frame();
        let body = self.body(frame.function)?;
        let mut operands = instruction.operands();

        match instruction.opcode() {
            // increment the linked counter selected by this function
            Opcode::PROFILE_INCREMENT => {
                let counter = operands.u32().map_err(|_| self.invalid_instruction())?;
                let counter = CounterId(body.counter_start + counter);
                let Some(profile) = self.profile.as_deref_mut() else {
                    unreachable!("profile instructions require an active profile");
                };

                profile.increment_counter(counter);
            }

            // sample the selected register under its linked sampler id
            Opcode::PROFILE_SAMPLE => {
                let sampler = operands.u32().map_err(|_| self.invalid_instruction())?;
                let sampler = SamplerId(body.sampler_start + sampler);
                let register = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let value = self.read(register.0).bits();
                let Some(profile) = self.profile.as_deref_mut() else {
                    unreachable!("profile instructions require an active profile");
                };

                profile.record_sample(sampler, value);
            }

            // reject impossible dispatch
            _ => unreachable!("profile dispatch selects one profile opcode"),
        }

        Ok(())
    }
}
