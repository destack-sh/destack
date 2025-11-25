use crate::{BuildResult, CompileTask, Compiler};

use dyst_dir::{ModuleId, Program};

/// task to build something into an artifact.
#[derive(Debug, Clone)]
pub enum BuildTask {
    /// Build a module.
    Build { module: ModuleId },
}

impl BuildTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Build { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Build { module } => {
                format!("build module '{module:?}'")
            }
        }
    }
}

impl From<BuildTask> for CompileTask {
    fn from(task: BuildTask) -> Self {
        CompileTask::Build(task)
    }
}

/// Output of a build task.
#[derive(Debug, Clone)]
pub struct BuildOutput {}

impl<'a> Compiler<'a> {
    /// Process a build task.
    pub fn process_build(&self, task: BuildTask) -> BuildResult<BuildOutput> {
        todo!("process_build({task:?})")
    }
}
