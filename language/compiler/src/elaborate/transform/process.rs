use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateResult, TaskDependencyError};

impl Compiler {
    /// Ensure a module has been transformed (post-analysis simplification).
    pub fn require_elaborate_module_transform(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::ElaborateTask;
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleTransform {
            module,
            profile,
        })
    }

    /// Transform a module (task entry point).
    pub(crate) fn elaborate_module_transform_phase(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<()> {
        self.require_analyze_module(module, profile)?;
        self.elaborate_module_transform(module, profile)?;
        Ok(())
    }
}
