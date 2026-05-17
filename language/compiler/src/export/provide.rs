use destack_artifact::ArtifactPayload;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::export::state::ExportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile)
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .dir_imported(module, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;

        // build expanded export inputs
        let view = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let bindings = expanded.binding_table(&bound);
        let dependencies = expanded.dependency_table(&imported);
        let mut state = ExportState::new(
            view,
            bound.namespace_scope,
            bindings,
            dependencies,
            self.strings(),
        );
        self.collect_exports(&mut state, &expanded.roots)
            .map_err(CompilerError::from)?;
        let (exported, diagnostics) = state.finish();
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirExported(exported))
    }
}
