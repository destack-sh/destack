use crate::build::FunctionBuilder;
use crate::{Instruction, Type, Value};

impl FunctionBuilder<'_> {
    /// Attempt to queue one runtime waiter with its completed value.
    pub fn waiter_queue(&mut self, waiter: Value, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::WaiterQueue {
            destination,
            waiter,
            value,
        });
        let boolean = self.tree.intern_type(Type::Boolean);
        self.define_value(destination, boolean);

        destination
    }

    /// Attempt to cancel one runtime waiter.
    pub fn waiter_cancel(&mut self, waiter: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::WaiterCancel {
            destination,
            waiter,
        });
        let boolean = self.tree.intern_type(Type::Boolean);
        self.define_value(destination, boolean);

        destination
    }
}
