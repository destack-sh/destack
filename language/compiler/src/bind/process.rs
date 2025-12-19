use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

use crate::{BindResult, Compiler, TaskDependencyError};

/// Task to bind AST into DIR (including syntactic desugaring).
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Bind)]
pub enum BindTask {
    /// Bind a module completely (build, desugar, validate).
    #[task(code = 1, trace = "module={module}")]
    BindModule { module: ModuleId },

    /// Build DIR for a module by binding its AST.
    #[task(code = 2, trace = "module={module}")]
    BindModuleBuild { module: ModuleId },

    /// Desugar the module syntactically.
    #[task(code = 3, trace = "module={module}")]
    BindModuleDesugar { module: ModuleId },

    /// Validate module "syntactic" correctness.
    #[task(code = 4, trace = "module={module}")]
    BindModuleValidate { module: ModuleId },
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => {
                self.require_bind_module_desugar(module)?;
            }
            BindTask::BindModuleBuild { module } => {
                self.require_import_module(module)?;
                let module = self.program.modules.get(module);
                // bind module roots
                let roots = {
                    let module = module.read();
                    self.bind_module_roots(&module, &module.ast)
                };
                {
                    let mut module = module.write();
                    module.dir.roots.extend(roots);
                };
                // bind module exports
                {
                    let mut module = module.write();
                    self.bind_module_exports(&mut module);
                }
            }
            BindTask::BindModuleDesugar { module } => {
                self.require_bind_module_build(module)?;
                let module = self.program.modules.get(module);
                let module = module.read();
                self.bind_module_desugar(&module);
            }
            BindTask::BindModuleValidate { module } => {
                self.require_bind_module_desugar(module)?;
                let module = self.program.modules.get(module);
                let module = module.read();
                self.validate_binding_names(&module);
                self.validate_binding_conflicts(&module);
            }
        }
        Ok(())
    }

    /// Ensure a module has been bound (DIR built).
    pub fn require_bind_module_build(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleBuild { module })
    }

    /// Ensure a module has been desugared after binding.
    pub fn require_bind_module_desugar(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleDesugar { module })
    }

    /// Ensure a module has been validated after binding.
    pub fn require_bind_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleValidate { module })
    }
}
