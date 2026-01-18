use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, ModuleStamp, ProfileStamp};
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateError, ElaborateResult, TaskDependencyError};

/// Task to elaborate a module: post-analysis transforms that need type information
/// but are target-independent.
///
/// Elaboration has two phases:
/// 1. **Transform** (simplify): reduce syntactic complexity within the same abstraction level
/// 2. **Reify** (make concrete): cross abstraction boundaries, turn abstract concepts into code
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Elaborate)]
pub enum ElaborateTask {
    /// Elaborate a module (umbrella task).
    #[task(code = 1, trace = "module={module} profile={profile}")]
    ElaborateModule {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Semantic transforms: simplify control flow and syntax.
    /// - Pattern matching → decision trees
    /// - Range expressions → iterator construction
    /// - Maybe/Must → explicit error handling
    #[task(code = 2, trace = "module={module} profile={profile}")]
    ElaborateModuleTransform {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Reification: make abstractions concrete.
    /// - Tree literals → constructor calls.
    /// - Operators → resolved method calls (based on Resolution).
    /// - Type descriptors → runtime type objects.
    /// - Implicit conversions → explicit cast nodes.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    ElaborateModuleReify {
        module: ModuleStamp,
        profile: ProfileStamp,
    },
}

impl Compiler {
    /// Process an elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<()> {
        match task {
            ElaborateTask::ElaborateModule { module, profile } => {
                self.ensure_module_profile_matches::<ElaborateError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.require_elaborate_module_reify(module.id, profile.id)?;
            }
            ElaborateTask::ElaborateModuleTransform { module, profile } => {
                self.ensure_module_profile_matches::<ElaborateError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.elaborate_module_transform(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            ElaborateTask::ElaborateModuleReify { module, profile } => {
                self.ensure_module_profile_matches::<ElaborateError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.elaborate_module_reify(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
                if self.is_code_module(module.id) {
                    self.stats.record_elaborate();
                }
            }
        }
        Ok(())
    }

    /// Ensure a module has been elaborated.
    pub fn require_elaborate_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ElaborateTask::ElaborateModule { module, profile })
    }
}
