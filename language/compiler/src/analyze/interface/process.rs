use crate::{AnalyzeResult, Compiler, CompilerContext};
use destack_artifact::DirInterface;
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use std::sync::Arc;

impl Compiler {
    /// Build interface state for a module by converging its canonical interface component.
    pub(crate) fn analyze_module_interface(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<Vec<(ModuleId, ProfileId, Arc<DirInterface>)>> {
        // ensure forward dependency edges are available for component discovery
        self.require_resolved_dependency_closure(context.revision(), [module_id], profile)?;

        // only the canonical anchor builds the shared component
        let anchor_module_id = self.interface_component_anchor_module_id(module_id, profile);
        if anchor_module_id != module_id {
            self.require_dir_interface(context.revision(), anchor_module_id, profile)?;
            return Ok(Vec::new());
        }

        self.analyze_interface_component(module_id, profile, context)
    }
}
