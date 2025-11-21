use crate::{Compiler, CompileTask, ExecuteResult};

use dyst_dir::{Expression, NodeId};

/// Task to execute something at compile time.
#[derive(Debug, Clone)]
pub enum ExecuteTask {
    /// Execute an Expression fully (in-place).
    ExecuteExpression { expression: NodeId<Expression> },
}

impl From<ExecuteTask> for CompileTask {
    fn from(task: ExecuteTask) -> Self {
        CompileTask::Execute(task)
    }
}

impl<'a> Compiler<'a> {
    /// Execute a node.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<()> {
        todo!("process_execute({task:?})")
    }
}
