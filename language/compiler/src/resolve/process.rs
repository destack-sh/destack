use crate::{Compiler, ResolveResult, Task, TaskOutput, TaskDebug};

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
}

impl TaskDebug for ResolveTask {
    fn name(&self) -> &'static str {
        match self {
            ResolveTask::ResolveModule { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            ResolveTask::ResolveModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone();
                format!("module={uri}")
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
