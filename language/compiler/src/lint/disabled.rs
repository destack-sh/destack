use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, ModuleStamp, PackageId, PackageStamp, ProfileStamp};
use destack_workspace::ProfileId;

use crate::{Compiler, LintError, LintResult, TaskDependencyError};

/// Task to lint something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lint)]
pub enum LintTask {
    /// Lint a module.
    #[task(code = 1, trace = "module={module} profile={profile}")]
    LintModule {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Lint a package
    #[task(code = 2, trace = "package={package}")]
    LintPackage { package: PackageStamp },
}

impl Compiler {
    /// Process a lint task.
    pub fn process_lint(&self, task: LintTask) -> LintResult<()> {
        // keep stale checks so task scheduling behavior stays stable
        match task {
            LintTask::LintModule { module, profile } => {
                self.ensure_module_profile_matches::<LintError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
            }
            LintTask::LintPackage { package } => {
                self.ensure_package_version_matches::<LintError>(package.id, package.version)?;
            }
        }

        Ok(())
    }

    /// Ensure a module has been linted.
    pub fn require_lint_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintModule {
            module: self.module_stamp(module_id),
            profile: self.profile_stamp(profile),
        })
    }

    /// Ensure a package has been linted.
    pub fn require_lint_package(&self, package_id: PackageId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintPackage {
            package: self.package_stamp(package_id),
        })
    }
}
