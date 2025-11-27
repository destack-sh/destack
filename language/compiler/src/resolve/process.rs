use crate::{TaskOutput, Task, Compiler, ResolveResult};

use dyst_dir::{ModuleId, Program};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ResolveTask {
    /// Resolve all unresolved nodes in a module.
    ResolveModule { module: ModuleId },
}

impl ResolveTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ResolveTask::ResolveModule { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            ResolveTask::ResolveModule { module } => {
                format!("resolve module '{module:?}'")
            }
        }
    }
}

impl From<ResolveTask> for Task {
    fn from(task: ResolveTask) -> Self {
        Task::Resolve(task)
    }
}

/// Output of a resolve task.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolveOutput {}

impl From<ResolveOutput> for TaskOutput {
    fn from(output: ResolveOutput) -> Self {
        TaskOutput::Resolve(output)
    }
}

impl Compiler {
    /// Process a resolve task.
    pub fn process_resolve(&self, task: ResolveTask) -> ResolveResult<ResolveOutput> {
        match task {
            ResolveTask::ResolveModule { module } => self.resolve_module(module)?,
        }
        Ok(ResolveOutput {})
    }
}
