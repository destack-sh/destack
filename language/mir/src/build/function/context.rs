use crate::build::FunctionBuilder;
use crate::{Instruction, TypeId, Value};

impl<'a> FunctionBuilder<'a> {
    /// Load the current execution context.
    pub fn context_current(&mut self, result_type: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ContextCurrent { destination });
        self.define_value(destination, result_type);

        destination
    }

    /// Replace the current execution context and return its previous value.
    pub fn context_replace(&mut self, context: Value, result_type: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ContextReplace {
            destination,
            context,
        });
        self.define_value(destination, result_type);

        destination
    }

    /// Extend one execution context with a variable value.
    pub fn context_bind(
        &mut self,
        context: Value,
        variable: Value,
        value: Value,
        node_type: TypeId,
        result_type: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ContextBind {
            destination,
            context,
            variable,
            value,
            node_type,
            result_type,
        });
        self.define_value(destination, result_type);

        destination
    }

    /// Load one variable value from an execution context.
    pub fn context_get(
        &mut self,
        context: Value,
        variable: Value,
        default: Value,
        node_type: TypeId,
        result_type: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ContextGet {
            destination,
            context,
            variable,
            default,
            node_type,
            result_type,
        });
        self.define_value(destination, result_type);

        destination
    }
}
