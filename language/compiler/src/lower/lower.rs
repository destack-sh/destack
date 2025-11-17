use crate::{Compiler, LowerResult};

/// Task to lower something.
#[derive(Debug, Clone)]
pub enum LowerTask {}

impl<'a> Compiler<'a> {
    /// Process a lower task.
    pub fn process_lower(&mut self, task: LowerTask) -> LowerResult<()> {
        todo!("process_lower({task:?})")
    }
}
