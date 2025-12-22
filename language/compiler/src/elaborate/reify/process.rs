use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult, TaskDependencyError};

impl Compiler {
    /// Ensure a module has been reified (abstractions made concrete).
    pub fn require_elaborate_module_reify(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        use crate::ElaborateTask;
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleReify { module })
    }

    /// Reify a module (task entry point).
    pub(crate) fn elaborate_module_reify_phase(&self, module: ModuleId) -> ElaborateResult<()> {
        self.require_elaborate_module_transform(module)?;
        self.elaborate_module_reify(module)?;
        Ok(())
    }
}
