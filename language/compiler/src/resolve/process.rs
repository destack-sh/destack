use crate::{Compiler, ResolveError, ResolveResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, ModuleStamp, ProfileStamp};
use destack_workspace::ProfileId;

/// Task to statically resolve something in-place.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Resolve)]
pub enum ResolveTask {
    /// Resolve all builtin modules and required language items.
    #[task(code = 0, trace = "builtins profile={profile}")]
    ResolveBuiltins { profile: ProfileStamp },

    /// Resolve a profile's libraries.
    #[task(code = 1, trace = "profile={profile}")]
    ResolveLibs { profile: ProfileStamp },

    /// Resolve a module completely (direct, canonical).
    #[task(code = 2, trace = "module={module} profile={profile}")]
    ResolveModule {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Resolve expressions and dependency items.
    /// Sets target_symbol for imports and populates namespace_exports.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    ResolveModuleDirect {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Prepare the per profile DIR for a module.
    #[task(code = 4, trace = "module={module} profile={profile}")]
    ResolveModulePrepare {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Compute canonical_symbol for all symbols.
    /// Follows target_symbol chains to find the canonical symbol.
    #[task(code = 5, trace = "module={module} profile={profile}")]
    ResolveModuleCanonical {
        module: ModuleStamp,
        profile: ProfileStamp,
    },
}

impl Compiler {
    /// Process a resolve task.
    pub fn process_resolve(&self, task: ResolveTask) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveBuiltins { profile } => {
                self.ensure_profile_version_matches::<ResolveError>(profile.id, profile.version)?;
                self.resolve_builtins(profile.id)?;
            }
            ResolveTask::ResolveLibs { profile } => {
                self.ensure_profile_version_matches::<ResolveError>(profile.id, profile.version)?;
                self.resolve_libs(profile.id)?;
            }
            ResolveTask::ResolveModule { module, profile } => {
                self.ensure_module_profile_matches::<ResolveError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.require_resolve_module_canonical(module.id, profile.id)?;
            }
            ResolveTask::ResolveModuleDirect { module, profile } => {
                self.ensure_module_profile_matches::<ResolveError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.resolve_module_direct(module.id, profile.id, module.version, profile.version)?;
            }
            ResolveTask::ResolveModulePrepare { module, profile } => {
                self.ensure_module_profile_matches::<ResolveError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.resolve_module_prepare(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            ResolveTask::ResolveModuleCanonical { module, profile } => {
                self.ensure_module_profile_matches::<ResolveError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.resolve_module_canonical(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
                if self.is_code_module(module.id) {
                    self.stats.record_resolve();
                }
            }
        }
        Ok(())
    }

    /// Ensure builtins have been resolved.
    pub fn require_resolve_builtins(&self, profile: ProfileId) -> Result<(), TaskDependencyError> {
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveBuiltins { profile })
    }

    /// Ensure a profile's libraries have been resolved.
    pub fn require_resolve_libs(&self, profile: ProfileId) -> Result<(), TaskDependencyError> {
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveLibs { profile })
    }

    /// Ensure a module's direct symbols have been resolved.
    pub fn require_resolve_module_direct(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveModuleDirect { module, profile })
    }

    /// Ensure a module's per profile DIR exists.
    pub fn require_resolve_module_prepare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveModulePrepare { module, profile })
    }

    /// Ensure another module's per profile DIR exists.
    pub fn require_resolve_module_prepare_if_needed(
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
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveModuleCanonical { module, profile })
    }

    /// Ensure a module has been resolved.
    pub fn require_resolve_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ResolveTask::ResolveModule { module, profile })
    }
}
