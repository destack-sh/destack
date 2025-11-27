use crate::{TaskOutput, Task, Compiler, ExecuteResult};

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
    pub fn message(&self, _program: &Program) -> String {
        match self {
            ExecuteTask::ExecuteExpression { node } => {
                format!("execute expression '{node:?}'")
            }
        }
    }
}

impl From<ExecuteTask> for Task {
    fn from(task: ExecuteTask) -> Self {
        Task::Execute(task)
    }
}

/// Output of an execute task.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecuteOutput {}

impl From<ExecuteOutput> for TaskOutput {
    fn from(output: ExecuteOutput) -> Self {
        TaskOutput::Execute(output)
    }
}

impl Compiler {
    /// Execute a node.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<ExecuteOutput> {
        todo!("process_execute({task:?})")
    }
}
