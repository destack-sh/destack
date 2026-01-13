use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, TaskDependencyError};

impl Compiler {
    /// Ensure a module has been reified (abstractions made concrete).
    pub fn require_elaborate_module_reify(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::ElaborateTask;
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleReify { module, profile })
    }

}
