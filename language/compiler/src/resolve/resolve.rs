use crate::{Compiler, CompilerTask, ResolveResult};

use dyst_dir::{ModuleId, Session};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone)]
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
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            ResolveTask::ResolveModule { module } => {
                format!("resolve module '{module:?}'")
            }
        }
    }
}

impl From<ResolveTask> for CompilerTask {
    fn from(task: ResolveTask) -> Self {
        CompilerTask::Resolve(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a resolve task.
    pub fn process_resolve(&mut self, task: ResolveTask) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveModule { module } => self.resolve_module(module),
        }
    }
}
