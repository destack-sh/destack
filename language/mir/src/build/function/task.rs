use crate::{Instruction, TypeId, Value};

use super::FunctionBuilder;

impl FunctionBuilder<'_> {
    /// Create one already completed task.
    pub fn task_resolve(&mut self, value: Value, task_type: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TaskResolve { destination, value });
        self.define_value(destination, task_type);

        destination
    }

    /// Start one ready continuation as a task.
    pub fn task_start(&mut self, continuation: Value, task_type: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TaskStart {
            destination,
            continuation,
        });
        self.define_value(destination, task_type);

        destination
    }

    /// Park one waiter until a task completes or is cancelled.
    pub fn task_park(&mut self, task: Value, waiter: Value) {
        self.insert_instruction(Instruction::TaskPark { task, waiter });
    }

    /// Request cooperative cancellation of one task.
    pub fn task_cancel(&mut self, task: Value) {
        self.insert_instruction(Instruction::TaskCancel { task });
    }

    /// Detach one task from its result.
    pub fn task_detach(&mut self, task: Value) {
        self.insert_instruction(Instruction::TaskDetach { task });
    }
}
