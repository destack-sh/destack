use std::iter;

use destack_artifact::{ArtifactPayload, ArtifactSidecar};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{FileContent, ModuleId};

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
        let profile_id = profile;
        let profile_state = self.profile(context.revision(), profile_id)?;
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile_id)
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .dir_imported(module, profile_id)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, profile_id)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;

        // build expanded export inputs
        let view = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let bindings = expanded.binding_table(&bound);
        let modules = expanded.module_table(&imported);
        let mut state = ExportState::new(
            view,
            module.as_ref(),
            &profile_state.key,
            profile_state.conditions(),
            bound.namespace_scope,
            bindings,
            modules,
            self.strings(),
        );
        self.collect_exports(&mut state, &expanded.roots)
            .map_err(CompilerError::from)?;
        let stats = state.stats;
        let (exported, diagnostics) = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "export")),
            FileContent::Text {
                content: stats.render_metadata(),
            },
        ));
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirExported(exported))
    }
}
