use crate::{AnalyzeResult, Compiler};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Route interface analysis tasks to component analysis.
    pub(crate) fn analyze_module_interface(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        _module_version: ModuleVersion,
        _profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        self.require_analyze_interface_component(module_id, profile)?;
        Ok(())
    }
}
