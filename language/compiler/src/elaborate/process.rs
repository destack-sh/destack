use crate::{TaskOutput, Task, Compiler, ElaborateResult};

use dyst_dir::{ModuleId, Program};

/// Task to elaborate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ElaborateTask {
    /// Elaborate a module.
    Elaborate { module: ModuleId },
}

impl ElaborateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Elaborate { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Elaborate { module } => {
                format!("elaborate module '{module:?}'")
            }
        }
    }
}

impl From<ElaborateTask> for Task {
    fn from(task: ElaborateTask) -> Self {
        Task::Elaborate(task)
    }
}

/// Output of an elaborate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ElaborateOutput {}

impl From<ElaborateOutput> for TaskOutput {
    fn from(output: ElaborateOutput) -> Self {
        TaskOutput::Elaborate(output)
    }
}

impl Compiler {
    /// Process a elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<ElaborateOutput> {
        todo!("process_elaborate({task:?})")
    }
}
