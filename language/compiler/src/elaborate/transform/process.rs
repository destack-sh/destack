use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateTask, TaskDependencyError};

impl Compiler {
    /// Ensure a module has been transformed (post-analysis simplification).
    pub fn require_elaborate_module_transform(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleTransform {
            module,
            profile,
        })
    }
}
