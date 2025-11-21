use crate::{Compiler, CompileTask, ElaborateResult};

/// Task to elaborate something.
#[derive(Debug, Clone)]
pub enum ElaborateTask {}

impl From<ElaborateTask> for CompileTask {
    fn from(task: ElaborateTask) -> Self {
        CompileTask::Elaborate(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<()> {
        todo!("process_elaborate({task:?})")
    }
}
