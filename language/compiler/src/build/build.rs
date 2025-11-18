use crate::{BuildResult, Compiler, CompilerTask};

/// task to build something into an artifact.
#[derive(Debug, Clone)]
pub enum BuildTask {}

impl From<BuildTask> for CompilerTask {
    fn from(task: BuildTask) -> Self {
        CompilerTask::Build(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a build task.
    pub fn process_build(&mut self, task: BuildTask) -> BuildResult<()> {
        todo!("process_build({task:?})")
    }
}
