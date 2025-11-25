use crate::{CompileOutput, CompileTask, Compiler, ValidateResult};

use dyst_dir::{ModuleId, Program};

/// Task to validate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValidateTask {
    /// Validate a module.
    Validate { module: ModuleId },
}

impl ValidateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Validate { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Validate { module } => {
                format!("validate module '{module:?}'")
            }
        }
    }
}

impl From<ValidateTask> for CompileTask {
    fn from(task: ValidateTask) -> Self {
        CompileTask::Validate(task)
    }
}

/// Output of a validate task.
#[derive(Debug, Clone)]
pub struct ValidateOutput {}

impl From<ValidateOutput> for CompileOutput {
    fn from(output: ValidateOutput) -> Self {
        CompileOutput::Validate(output)
    }
}

impl Compiler {
    /// Process a validate task.
    pub fn process_validate(&self, task: ValidateTask) -> ValidateResult<ValidateOutput> {
        todo!("process_validate({task:?})")
    }
}
