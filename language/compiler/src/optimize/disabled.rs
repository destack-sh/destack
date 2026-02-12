use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, ModuleStamp, PackageStamp, ProfileStamp};
use destack_workspace::{ProfileId, ProgramStamp, TargetId};

use crate::{Compiler, OptimizeError, OptimizeResult, TaskDependencyError};

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Optimize)]
pub enum OptimizeTask {
    /// Optimize a module's MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    OptimizeModule {
        /// The module to optimize.
        module: ModuleStamp,
        /// The profile to optimize.
        profile: ProfileStamp,
        /// The target to optimize.
        target: TargetId,
    },

    /// Optimize all modules in a package for a target.
    #[task(code = 2, trace = "package={package} target={target}")]
    OptimizePackage {
        /// The package id to optimize.
        package: PackageStamp,
        /// The target id for this package.
        target: TargetId,
    },

    /// Optimize all modules in the program for a target name.
    #[task(code = 3, trace = "target={target}")]
    OptimizeProgram {
        /// The target name to optimize.
        target: String,
        /// Stamp of the current package versions.
        program_stamp: ProgramStamp,
    },
}

impl Compiler {
    /// Process an optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<()> {
        // keep dependency and stale checks stable when optimize is disabled
        match task {
            OptimizeTask::OptimizeModule {
                module,
                profile,
                target,
            } => {
                self.ensure_module_profile_matches::<OptimizeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;

                self.require_lower_module(module.id, profile.id, &target)?;
            }
            OptimizeTask::OptimizePackage { package, .. } => {
                self.ensure_package_version_matches::<OptimizeError>(package.id, package.version)?;
            }
            OptimizeTask::OptimizeProgram { program_stamp, .. } => {
                self.ensure_program_stamp_matches::<OptimizeError>(program_stamp)?;
            }
        }

        Ok(())
    }

    /// Ensure a module has been optimized.
    pub fn require_optimize(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(OptimizeTask::OptimizeModule {
            module: self.module_stamp(module),
            profile: self.profile_stamp(profile),
            target: target.clone(),
        })
    }
}
