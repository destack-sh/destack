use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

use crate::{BindResult, Compiler, TaskDependencyError};

use destack_workspace::Module;

/// Task to bind AST into DIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Bind)]
pub enum BindTask {
    /// Bind a module completely (build, validate).
    #[task(code = 1, trace = "module={module}")]
    BindModule { module: ModuleId },

    /// Build DIR for a module by binding its AST.
    #[task(code = 2, trace = "module={module}")]
    BindModuleBuild { module: ModuleId },

    /// Validate module semantics that depend on structural context (but not types).
    #[task(code = 3, trace = "module={module}")]
    BindModuleValidate { module: ModuleId },
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => {
                self.require_bind_module_build(module)?;
            }
            BindTask::BindModuleBuild { module } => {
                self.require_import_module(module)?;
                let module = self.program.modules.get(module);
                let mut module = module.write();
                self.bind_module_build(&mut module);
            }
            BindTask::BindModuleValidate { module } => {
                self.require_bind_module_build(module)?;
                let module = self.program.modules.get(module);
                let module = module.read();
                self.bind_module_validate(&module);
            }
        }
        Ok(())
    }

    /// Ensure a module has been bound (DIR built).
    pub fn require_bind_module_build(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleBuild { module })
    }

    /// Ensure a module has been validated after binding.
    pub fn require_bind_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleValidate { module })
    }

    /// Bind a module and build its DIR.
    fn bind_module_build(&self, module: &mut Module) {
        // bind AST into DIR
        self.bind_module_roots(module);

        // bind module exports
        self.bind_module_exports(module);
    }

    /// Validate a module after binding into DIR.
    fn bind_module_validate(&self, module: &Module) {
        // check for reserved identifiers
        self.validate_binding_names(module);

        // check for conflicting bindings
        self.validate_binding_conflicts(module);
    }
}
