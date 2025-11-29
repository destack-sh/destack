use crate::{BuildResult, Compiler, Task, TaskDebug, TaskOutput};

use destack_dir::{ModuleId, Program};

/// task to build something into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
}

impl TaskDebug for BuildTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Build { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Build { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<BuildTask> for Task {
    fn from(task: BuildTask) -> Self {
        Task::Build(task)
    }
}

/// Output of a build task.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildOutput {}

impl From<BuildOutput> for TaskOutput {
    fn from(output: BuildOutput) -> Self {
        TaskOutput::Build(output)
    }
}

impl Compiler {
    /// Process a build task.
    pub fn process_build(&self, task: BuildTask) -> BuildResult<BuildOutput> {
        todo!("process_build({task:?})")
    }
}
