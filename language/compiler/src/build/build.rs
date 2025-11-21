use crate::{BuildResult, Compiler, CompileTask};

/// task to build something into an artifact.
#[derive(Debug, Clone)]
pub enum BuildTask {}

impl From<BuildTask> for CompileTask {
    fn from(task: BuildTask) -> Self {
        CompileTask::Build(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a build task.
    pub fn process_build(&self, task: BuildTask) -> BuildResult<()> {
        todo!("process_build({task:?})")
    }
}
