use crate::{Compiler, ResolveResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

/// Task to statically resolve something in-place.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Resolve)]
pub enum ResolveTask {
    /// Resolve all builtin modules and required language items.
    #[task(code = 0, trace = "builtins profile={profile}")]
    ResolveBuiltins { profile: ProfileId },

    /// Resolve a profile's libraries.
    #[task(code = 1, trace = "profile={profile}")]
    ResolveLibs { profile: ProfileId },

    /// Resolve a module completely (direct, canonical).
    #[task(code = 2, trace = "module={module} profile={profile}")]
    ResolveModule {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Resolve expressions and dependency items.
    /// Sets target_symbol for imports and populates namespace_exports.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    ResolveModuleDirect {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Prepare the per profile DIR for a module.
    #[task(code = 4, trace = "module={module} profile={profile}")]
    ResolveModulePrepare {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Compute canonical_symbol for all symbols.
    /// Follows target_symbol chains to find the canonical symbol.
    #[task(code = 5, trace = "module={module} profile={profile}")]
    ResolveModuleCanonical {
        module: ModuleId,
        profile: ProfileId,
    },
}

impl Compiler {
    /// Process a resolve task.
    pub fn process_resolve(&self, task: ResolveTask) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveBuiltins { profile } => {
                self.resolve_builtins(profile)?;
            }
            ResolveTask::ResolveLibs { profile } => {
                self.resolve_libs(profile)?;
            }
            ResolveTask::ResolveModule { module, profile } => {
                self.require_resolve_module_canonical(module, profile)?;
            }
            ResolveTask::ResolveModuleDirect { module, profile } => {
                self.require_resolve_module_prepare(module, profile)?;
                self.resolve_module_direct(module, profile)?;
            }
            ResolveTask::ResolveModulePrepare { module, profile } => {
                self.require_bind_module_validate(module)?;
                self.resolve_module_prepare(module, profile)?;
            }
            ResolveTask::ResolveModuleCanonical { module, profile } => {
                self.require_resolve_module_direct(module, profile)?;
                self.resolve_module_canonical(module, profile)?;
            }
        }
        Ok(())
    }

    /// Ensure builtins have been resolved.
    pub fn require_resolve_builtins(&self, profile: ProfileId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveBuiltins { profile })
    }

    /// Ensure a profile's libraries have been resolved.
    pub fn require_resolve_libs(&self, profile: ProfileId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveLibs { profile })
    }

    /// Ensure a module's direct symbols have been resolved.
    pub fn require_resolve_module_direct(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModuleDirect { module, profile })
    }

    /// Ensure a module's per profile DIR exists.
    pub fn require_resolve_module_prepare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModulePrepare { module, profile })
    }

    /// Ensure another module's per profile DIR exists.
    pub fn require_resolve_module_prepare_if_other(
        &self,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        if module == other {
            return Ok(());
        }
        self.require_resolve_module_prepare(other, profile)
    }

    /// Ensure a different module's direct symbols have been resolved.
    pub fn require_resolve_module_direct_if_other(
        &self,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        if module == other {
            return Ok(());
        }
        self.require_resolve_module_direct(other, profile)
    }

    /// Ensure a module's canonical symbols have been resolved.
    pub fn require_resolve_module_canonical(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModuleCanonical { module, profile })
    }

    /// Ensure a module has been resolved.
    pub fn require_resolve_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ResolveTask::ResolveModule { module, profile })
    }
}
