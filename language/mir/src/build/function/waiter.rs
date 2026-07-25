use crate::build::FunctionBuilder;
use crate::{Instruction, Value};

impl FunctionBuilder<'_> {
    /// Queue one runtime waiter with its completed value.
    pub fn waiter_queue(&mut self, waiter: Value, value: Value) {
        self.insert_instruction(Instruction::WaiterQueue { waiter, value });
    }

    /// Cancel one runtime waiter.
    pub fn waiter_cancel(&mut self, waiter: Value) {
        self.insert_instruction(Instruction::WaiterCancel { waiter });
    }
}
