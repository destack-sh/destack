use crate::{Compiler, CompilerTask, ExecuteResult};

use dyst_dir::{Expression, NodeId};

/// Task to execute something at compile time.
#[derive(Debug, Clone)]
pub enum ExecuteTask {
    /// Execute an Expression fully (in-place).
    ExecuteExpression { expression: NodeId<Expression> },
}

impl From<ExecuteTask> for CompilerTask {
    fn from(task: ExecuteTask) -> Self {
        CompilerTask::Execute(task)
    }
}

impl<'a> Compiler<'a> {
    /// Execute a node.
    pub fn process_execute(&mut self, task: ExecuteTask) -> ExecuteResult<()> {
        todo!("process_execute({task:?})")
    }
}
