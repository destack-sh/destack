use crate::{Compiler, CompilerTask, ValidateResult};

/// Task to validate something.
#[derive(Debug, Clone)]
pub enum ValidateTask {}

impl From<ValidateTask> for CompilerTask {
    fn from(task: ValidateTask) -> Self {
        CompilerTask::Validate(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a validate task.
    pub fn process_validate(&self, task: ValidateTask) -> ValidateResult<()> {
        todo!("process_validate({task:?})")
    }
}
