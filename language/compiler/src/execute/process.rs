use crate::{CompileOutput, CompileTask, Compiler, ExecuteResult};

use dyst_dir::{LocalNodeIdAny, Program};

/// Task to execute something at compile time.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExecuteTask {
    /// Execute an Expression fully (in-place).
    ExecuteExpression { node: LocalNodeIdAny },
}

impl ExecuteTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ExecuteTask::ExecuteExpression { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            ExecuteTask::ExecuteExpression { node } => {
                format!("execute expression '{node:?}'")
            }
        }
    }
}

impl From<ExecuteTask> for CompileTask {
    fn from(task: ExecuteTask) -> Self {
        CompileTask::Execute(task)
    }
}

/// Output of an execute task.
#[derive(Debug, Clone)]
pub struct ExecuteOutput {}

impl From<ExecuteOutput> for CompileOutput {
    fn from(output: ExecuteOutput) -> Self {
        CompileOutput::Execute(output)
    }
}

impl<'a> Compiler<'a> {
    /// Execute a node.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<ExecuteOutput> {
        todo!("process_execute({task:?})")
    }
}
