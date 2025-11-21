use crate::{CompileTask, Compiler, ValidateResult};

/// Task to validate something.
#[derive(Debug, Clone)]
pub enum ValidateTask {}

impl From<ValidateTask> for CompileTask {
    fn from(task: ValidateTask) -> Self {
        CompileTask::Validate(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a validate task.
    pub fn process_validate(&self, task: ValidateTask) -> ValidateResult<()> {
        todo!("process_validate({task:?})")
    }
}
