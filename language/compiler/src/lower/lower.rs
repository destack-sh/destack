use crate::{Compiler, CompilerTask, LowerResult};

/// Task to lower something.
#[derive(Debug, Clone)]
pub enum LowerTask {}

impl From<LowerTask> for CompilerTask {
    fn from(task: LowerTask) -> Self {
        CompilerTask::Lower(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a lower task.
    pub fn process_lower(&mut self, task: LowerTask) -> LowerResult<()> {
        todo!("process_lower({task:?})")
    }
}
