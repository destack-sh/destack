use crate::{Compiler, Task, TaskOutput, TaskDebug, ValidateResult};

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
}

impl TaskDebug for ValidateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Validate { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Validate { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone();
                format!("module={uri}")
            }
        }
    }
}

impl From<ValidateTask> for Task {
    fn from(task: ValidateTask) -> Self {
        Task::Validate(task)
    }
}

/// Output of a validate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidateOutput {}

impl From<ValidateOutput> for TaskOutput {
    fn from(output: ValidateOutput) -> Self {
        TaskOutput::Validate(output)
    }
}

impl Compiler {
    /// Process a validate task.
    pub fn process_validate(&self, task: ValidateTask) -> ValidateResult<ValidateOutput> {
        todo!("process_validate({task:?})")
    }
}
