use crate::{TaskOutput, Task, Compiler, LowerResult};

use dyst_dir::{ModuleId, Program};

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LowerTask {
    /// Lower a module.
    Lower { module: ModuleId },
}

impl LowerTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Lower { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Lower { module } => {
                format!("lower module '{module:?}'")
            }
        }
    }
}

impl From<LowerTask> for Task {
    fn from(task: LowerTask) -> Self {
        Task::Lower(task)
    }
}

/// Output of a lower task.
#[derive(Debug, Clone, PartialEq)]
pub struct LowerOutput {}

impl From<LowerOutput> for TaskOutput {
    fn from(output: LowerOutput) -> Self {
        TaskOutput::Lower(output)
    }
}

impl Compiler {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<LowerOutput> {
        todo!("process_lower({task:?})")
    }
}
