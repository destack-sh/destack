use crate::{BindResult, Compiler, TaskDependencyError};
use destack_source::ModuleId;

impl Compiler {
    /// Ensure a module has been desugared after binding.
    pub fn require_bind_module_desugar(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        use crate::BindTask;
        self.do_require_task_internal_only(BindTask::BindModuleDesugar { module })
    }

    /// Desugar a module syntactically.
    pub(crate) fn bind_module_desugar_phase(&self, module: ModuleId) -> BindResult<()> {
        self.require_bind_module_build(module)?;
        let module = self.program.modules.get(module);
        let module = module.read();
        self.bind_module_desugar(&module);
        Ok(())
    }
}
