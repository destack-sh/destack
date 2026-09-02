use crate::{CounterId, Instruction, SamplerId, Value};

use super::FunctionBuilder;

impl FunctionBuilder<'_> {
    /// Increment one profile counter.
    pub fn profile_increment(&mut self, counter: CounterId) {
        self.insert_instruction(Instruction::ProfileIncrement { counter });
    }

    /// Record one value under a profile sampler.
    pub fn profile_sample(&mut self, sampler: SamplerId, value: Value) {
        self.insert_instruction(Instruction::ProfileSample { sampler, value });
    }
}
