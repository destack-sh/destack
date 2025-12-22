use crate::{Compiler, ResolveResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

/// Task to statically resolve something in-place.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Resolve)]
pub enum ResolveTask {
    /// Resolve all builtin modules and required language items.
    #[task(code = 0, trace = "builtins")]
    ResolveBuiltins,

    /// Resolve a profile's libraries.
    #[task(code = 1, trace = "profile={profile}")]
    ResolveLibs { profile: ProfileId },

    /// Resolve a module completely (direct, canonical).
    #[task(code = 2, trace = "module={module}")]
    ResolveModule { module: ModuleId },

    /// Resolve expressions and dependency items (phase 1).
    /// Sets target_symbol for imports and populates namespace_exports.
    #[task(code = 3, trace = "module={module}")]
    ResolveModuleDirect { module: ModuleId },

    /// Compute canonical_symbol for all symbols (phase 2).
    /// Follows target_symbol chains to find the canonical symbol.
    #[task(code = 4, trace = "module={module}")]
    ResolveModuleCanonical { module: ModuleId },
}

impl Compiler {
    /// Process a resolve task.
    pub fn process_resolve(&self, task: ResolveTask) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveBuiltins => {
                self.resolve_builtins()?;
            }
            ResolveTask::ResolveLibs { profile } => {
                self.resolve_libs(profile)?;
            }
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
        Ok(())
    }

    /// Ensure builtins have been resolved.
    pub fn require_resolve_builtins(&self) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveBuiltins)
    }

    /// Ensure a profile's libraries have been resolved.
    pub fn require_resolve_libs(&self, profile: ProfileId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveLibs { profile })
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

    /// Ensure a module has been resolved.
    pub fn require_resolve_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModule { module })
    }
}
