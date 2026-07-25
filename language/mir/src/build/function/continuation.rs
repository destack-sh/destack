use crate::build::FunctionBuilder;
use crate::{Block, BlockTarget, Function, Instruction, LocalNodeId, Terminator, Type, Value};

#[allow(clippy::too_many_arguments)]
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
    pub fn resume(
        &mut self,
        continuation: Value,
        command: Value,
        yielded_block: LocalNodeId<Block>,
        yielded_arguments: Vec<Value>,
        returned_block: LocalNodeId<Block>,
        returned_arguments: Vec<Value>,
        unwind: Option<(LocalNodeId<Block>, Vec<Value>)>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, yielded_block);
        self.add_predecessor(block, returned_block);
        if let Some((unwind, _)) = &unwind {
            self.add_predecessor(block, *unwind);
        }

        let yielded_arguments = self.tree.add_values(&yielded_arguments);
        let returned_arguments = self.tree.add_values(&returned_arguments);
        let yielded = BlockTarget::new(yielded_block, yielded_arguments);
        let returned = BlockTarget::new(returned_block, returned_arguments);
        let unwind = unwind
            .map(|(block, arguments)| BlockTarget::new(block, self.tree.add_values(&arguments)));
        let terminator = self.tree.get(block).terminator;
        *self.tree.get_mut(terminator) = Terminator::Resume {
            continuation,
            command,
            yielded,
            returned,
            unwind,
        };
    }
}
