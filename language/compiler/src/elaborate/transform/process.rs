use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult, TaskDependencyError};

impl Compiler {
    /// Ensure a module has been transformed (post-analysis simplification).
    pub fn require_elaborate_module_transform(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        use crate::ElaborateTask;
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleTransform { module })
    }

    /// Transform a module (task entry point).
    pub(crate) fn elaborate_module_transform_phase(&self, module: ModuleId) -> ElaborateResult<()> {
        self.require_analyze_module(module)?;
        self.elaborate_module_transform(module)?;
        Ok(())
    }
}
