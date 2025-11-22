use crate::{CompileTask, Compiler, LinkResult};

/// Task to link something.
#[derive(Debug, Clone)]
pub enum LinkTask {}

impl From<LinkTask> for CompileTask {
    fn from(task: LinkTask) -> Self {
        CompileTask::Link(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<()> {
        todo!("process_link({task:?})")
    }
}
