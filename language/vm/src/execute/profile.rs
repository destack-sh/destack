use tspp_bytecode::{Instruction, Opcode};
use tspp_program::{CounterId, Runtime, SamplerId};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one explicit profile operation.
    pub(crate) fn execute_profile(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);

        match instruction.opcode() {
            // increment the linked counter selected by this function
            Opcode::PROFILE_INCREMENT => {
                let counter = operands.u32()?;
                let counter = CounterId(counter);
                let Some(profile) = self.profile.as_deref_mut() else {
                    unreachable!("profile instructions require an active profile");
                };

                profile.increment_counter(counter);
            }

            // sample the selected register under its linked sampler id
            Opcode::PROFILE_SAMPLE => {
                let sampler = operands.u32()?;
                let sampler = SamplerId(sampler);
                let register = operands.register()?;
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
