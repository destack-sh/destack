use crate::{AnalyzeResult, Compiler, TaskDependencyError};
use destack_dir::{Declaration, Member, Parameter};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module has been validated after analysis.
    pub fn require_analyze_module_validate(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module, profile })
    }

    /// Ensure a module has been fully analyzed (including checks).
    pub fn require_analyze_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module, profile })
    }

    /// Phase 3: Final validation checks.
    pub(crate) fn analyze_module_validate(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let types = dir.types.read();

        // validate declarations
        for (id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            self.validate_declaration(&module, &types, id, declaration);
        }

        // validate parameters
        for (id, parameter) in tree.iter_nodes_of_type::<Parameter>() {
            self.validate_parameter(&module, &tree, id, parameter);
        }

        // validate members
        for (id, member) in tree.iter_nodes_of_type::<Member>() {
            self.validate_member(&module, &tree, id, member);
        }

        Ok(())
    }
}
