use crate::{CompileTask, Compiler, LowerResult};

/// Task to lower something.
#[derive(Debug, Clone)]
pub enum LowerTask {}

impl From<LowerTask> for CompileTask {
    fn from(task: LowerTask) -> Self {
        CompileTask::Lower(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<()> {
        todo!("process_lower({task:?})")
    }
}
