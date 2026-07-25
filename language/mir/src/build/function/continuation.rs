use crate::build::FunctionBuilder;
use crate::{Function, Instruction, LocalNodeId, Type, Value};

impl<'a> FunctionBuilder<'a> {
    /// Create one ready continuation for a coroutine invocation.
    pub fn continuation_new(
        &mut self,
        function: LocalNodeId<Function>,
        arguments: Vec<Value>,
        continuation_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::ContinuationNew {
            destination,
            function,
            arguments,
        });
        self.define_value(destination, continuation_type);

        destination
    }

    /// Resume one continuation until it yields or returns.
    pub fn continuation_resume(
        &mut self,
        continuation: Value,
        command: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ContinuationResume {
            destination,
            continuation,
            command,
        });
        self.define_value(destination, result_type);

        destination
    }
}
