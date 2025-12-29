use crate::{Compiler, GenerateError, GenerateResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::{OutputFormat, TargetId};

/// Task to generate code for a module into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Generate)]
pub enum GenerateTask {
    /// Generate a module for a specific target.
    #[task(code = 1, trace = "module={module} target={target}")]
    GenerateModule {
        /// The module to generate.
        module: ModuleId,
        /// The target id (for the module's package).
        target: TargetId,
    },
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<()> {
        match task {
            GenerateTask::GenerateModule { module, target } => {
                self.generate_module(module, &target)?;
                self.stats.record_generate();
            }
        }
        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(&self, module_id: ModuleId, target_id: &TargetId) -> GenerateResult<()> {
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

        let profile = self
            .program
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: format!("profile not found for target '{}'", target_id.name),
            })?;

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
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(GenerateTask::GenerateModule {
            module,
            target: target.clone(),
        })
    }
}
