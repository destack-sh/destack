use crate::{Compiler, GenerateError, GenerateResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_dir::{GlobalNodeIdAny, LocalNodeIdAny, NodeType};
use destack_source::ModuleId;
use destack_workspace::OutputFormat;

/// Task to generate code for a module into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Generate)]
pub enum GenerateTask {
    /// Generate a module for a specific target.
    #[task(code = 1, trace = "module={module} target={target}")]
    GenerateModule {
        /// The module to generate.
        module: ModuleId,
        /// The target name (looked up on the module's package).
        target: String,
    },
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<()> {
        match task {
            GenerateTask::GenerateModule { module, target } => {
                self.generate_module(module, &target)?;
            }
        }
        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(&self, module_id: ModuleId, target_name: &str) -> GenerateResult<()> {
        // look up target from module's package
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package.targets.get(target_name).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            node: Self::placeholder_node(module_id),
            message: format!("target '{target_name}' not found"),
        })?;

        // dispatch based on output format
        match target.output {
            OutputFormat::Js | OutputFormat::Ts => self.generate_js(module_id, &target),
            OutputFormat::Native | OutputFormat::Wasm => {
                self.generate_cranelift(module_id, &target)
            }
        }
    }

    /// Create a placeholder node for errors without a specific location.
    /// nocheckin #Broken: don't do placeholder nodes in generate
    pub(super) fn placeholder_node(module_id: ModuleId) -> GlobalNodeIdAny {
        GlobalNodeIdAny::new(
            module_id,
            LocalNodeIdAny {
                id: 0,
                ty: NodeType::Expression,
            },
        )
    }

    /// Ensure a module has been generated.
    pub fn require_generate_module(
        &self,
        module: ModuleId,
        target: &str,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(GenerateTask::GenerateModule {
            module,
            target: target.to_string(),
        })
    }
}
