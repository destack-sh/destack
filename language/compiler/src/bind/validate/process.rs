use crate::{BindResult, Compiler, TaskDependencyError};
use destack_source::ModuleId;

impl Compiler {
    /// Ensure a module has been validated after binding.
    pub fn require_bind_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        use crate::BindTask;
        self.do_require_task_internal_only(BindTask::BindModuleValidate { module })
    }

    /// Validate module "syntactic" correctness.
    pub(crate) fn bind_module_validate(&self, module: ModuleId) -> BindResult<()> {
        self.require_bind_module_desugar(module)?;
        let module = self.program.modules.get(module);
        let module = module.read();
        self.validate_binding_names(&module);
        self.validate_binding_conflicts(&module);
        Ok(())
    }
}
