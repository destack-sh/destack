use crate::{CompileTask, Compiler, ResolveResult};

use dyst_dir::{ModuleId, NodeTree, Session};

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

impl From<ResolveTask> for CompileTask {
    fn from(task: ResolveTask) -> Self {
        CompileTask::Resolve(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a resolve task.
    pub fn process_resolve(&self, task: ResolveTask, tree: &mut NodeTree) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveModule { module } => self.resolve_module(module, tree),
        }
    }
}
