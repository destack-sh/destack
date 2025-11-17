use crate::{Compiler, OptimizeResult};

/// Task to optimize something.
#[derive(Debug, Clone)]
pub enum OptimizeTask {}

impl<'a> Compiler<'a> {
    /// Process a optimize task.
    pub fn process_optimize(&mut self, task: OptimizeTask) -> OptimizeResult<()> {
        todo!("process_optimize({task:?})")
    }
}
