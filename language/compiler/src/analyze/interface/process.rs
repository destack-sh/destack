use crate::{AnalyzeResult, Compiler};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Build interface state for a module by converging its canonical interface component.
    pub(crate) fn analyze_module_interface(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // ensure forward dependency edges are available for component discovery
        self.require_interface_forward_closure(module_id, profile)?;

        // only the canonical anchor builds the shared component
        let anchor_module_id = self.interface_component_anchor_module_id(module_id, profile);
        if anchor_module_id != module_id {
            self.require_dir_interface(anchor_module_id, profile)?;
            return Ok(());
        }

        let graph_version = self.module_graph_version(profile);
        self.analyze_interface_component(
            module_id,
            profile,
            module_version,
            profile_version,
            graph_version,
        )?;
        Ok(())
    }
}
