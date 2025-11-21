use crate::{Compiler, CompileTask, OptimizeResult};

/// Task to optimize something.
#[derive(Debug, Clone)]
pub enum OptimizeTask {}

impl From<OptimizeTask> for CompileTask {
    fn from(task: OptimizeTask) -> Self {
        CompileTask::Optimize(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<()> {
        todo!("process_optimize({task:?})")
    }
}
