use crate::{Compiler, LinkResult};

/// Task to link something.
#[derive(Debug, Clone)]
pub enum LinkTask {}

impl<'a> Compiler<'a> {
    /// Process a link task.
    pub fn process_link(&mut self, task: LinkTask) -> LinkResult<()> {
        todo!("process_link({task:?})")
    }
}
