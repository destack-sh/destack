use crate::{AnalyzeResult, Compiler};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Route export analysis tasks to the export module.
    pub(crate) fn analyze_module_export(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        self.analyze_module_export_inner(module_id, profile, module_version, profile_version)
    }
}
