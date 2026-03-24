use std::path::Path;

use crate::Compiler;

use super::plan::ScriptLinkPlan;

impl Compiler {
    /// Build one minimal source map payload for a linked script target.
    pub(crate) fn linked_script_source_map(
        &self,
        package_dir: &Path,
        link_plan: &ScriptLinkPlan,
    ) -> destack_artifact::SourceMapArtifact {
        let sources = link_plan
            .modules
            .iter()
            .map(|module_id| self.package_relative_module_path(package_dir, *module_id))
            .collect::<Vec<_>>();

        destack_artifact::SourceMapArtifact {
            sources,
            ..destack_artifact::SourceMapArtifact::empty(String::new())
        }
    }
}
