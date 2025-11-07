use crate::Compiler;

use dyst_dir::{Expression, NodeId};

/// Request to execute something at compile time.
#[derive(Debug, Clone)]
pub enum ExecuteTask {
    /// Execute an Expression fully (in-place).
    ExecuteExpression { expression: NodeId<Expression> },
}

impl<'a> Compiler<'a> {
    /// Execute a node.
    pub fn process_execute(&mut self, request: ExecuteTask) {
        todo!("process_execute({request:?})")
    }
}
