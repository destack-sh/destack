use crate::Compiler;

/// Task to optimize something.
#[derive(Debug, Clone)]
pub enum OptimizeTask {}

impl<'a> Compiler<'a> {
    /// Process a optimize task.
    pub fn process_optimize(&mut self, task: OptimizeTask) {
        todo!("process_optimize({task:?})")
    }
}
