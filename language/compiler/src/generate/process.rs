use crate::{Compiler, GenerateError, GenerateResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, ModuleStamp, ModuleVersion, ProfileStamp, ProfileVersion};
use destack_workspace::{OutputFormat, ProfileId, TargetId};

/// Task to generate code for a module into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Generate)]
pub enum GenerateTask {
    /// Generate a module for a specific target.
    #[task(code = 1, trace = "module={module} target={target}")]
    GenerateModule {
        /// The module stamp to generate.
        module: ModuleStamp,
        /// The profile stamp for generation.
        profile: ProfileStamp,
        /// The target id (for the module's package).
        target: TargetId,
    },
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<()> {
        match task {
            GenerateTask::GenerateModule {
                module,
                profile,
                target,
            } => {
                self.ensure_module_profile_matches::<GenerateError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.generate_module(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                    &target,
                )?;
                self.stats.record_generate();
            }
        }
        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target_id: &TargetId,
    ) -> GenerateResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<GenerateError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;

        // look up target from module's package
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package.targets.get(target_id).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            module: module_id,
            message: format!("target '{}' not found", target_id.name),
        })?;

        let resolved_profile = self
            .program
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: format!("profile not found for target '{}'", target_id.name),
            })?;
        if resolved_profile != profile {
            return Ok(());
        }

        self.require_execute_module_patch(module_id, profile)?;

        // dispatch based on output format
        match target.output {
            OutputFormat::Js | OutputFormat::Ts => self.generate_js(module_id, &target, profile),
            OutputFormat::Native | OutputFormat::Wasm => {
                self.generate_cranelift(module_id, &target, profile)
            }
        }
    }

    /// Ensure a module has been generated.
    pub fn require_generate_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(GenerateTask::GenerateModule {
            module,
            profile,
            target: target.clone(),
        })
    }
}
