use crate::{Compiler, ResolveResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to statically resolve something in-place.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ResolveTask {
    /// Resolve a module completely (direct, canonical).
    ResolveModule { module: ModuleId },

    /// Resolve expressions and dependency items (phase 1).
    /// Sets target_symbol for imports and populates namespace_exports.
    ResolveModuleDirect { module: ModuleId },

    /// Compute canonical_symbol for all symbols (phase 2).
    /// Follows target_symbol chains to find the canonical symbol.
    ResolveModuleCanonical { module: ModuleId },
}

impl ResolveTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ResolveTask::ResolveModule { .. } => 1,
            ResolveTask::ResolveModuleDirect { .. } => 2,
            ResolveTask::ResolveModuleCanonical { .. } => 3,
        }
    }
}

impl TaskDebug for ResolveTask {
    fn name(&self) -> &'static str {
        match self {
            ResolveTask::ResolveModule { .. } => "module",
            ResolveTask::ResolveModuleDirect { .. } => "module_direct",
            ResolveTask::ResolveModuleCanonical { .. } => "module_canonical",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            ResolveTask::ResolveModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            ResolveTask::ResolveModuleDirect { module }
            | ResolveTask::ResolveModuleCanonical { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
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
            ResolveTask::ResolveModule { module } => {
                self.require_resolve_module_canonical(module)?;
            }
            ResolveTask::ResolveModuleDirect { module } => {
                self.require_bind_module_validate(module)?;
                self.resolve_module_direct(module)?;
            }
            ResolveTask::ResolveModuleCanonical { module } => {
                self.require_resolve_module_direct(module)?;
                self.resolve_module_canonical(module)?;
            }
        }
        Ok(ResolveOutput {})
    }

    /// Ensure a module's direct symbols have been resolved (phase 1).
    pub fn require_resolve_module_direct(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModuleDirect { module })
    }

    /// Ensure a module's canonical symbols have been resolved (phase 2).
    pub fn require_resolve_module_canonical(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModuleCanonical { module })
    }
}
