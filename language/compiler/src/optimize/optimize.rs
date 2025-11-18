use crate::{Compiler, CompilerTask, OptimizeResult};

/// Task to optimize something.
#[derive(Debug, Clone)]
pub enum OptimizeTask {}

impl From<OptimizeTask> for CompilerTask {
    fn from(task: OptimizeTask) -> Self {
        CompilerTask::Optimize(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a optimize task.
    pub fn process_optimize(&mut self, task: OptimizeTask) -> OptimizeResult<()> {
        todo!("process_optimize({task:?})")
    }
}
