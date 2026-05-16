use destack_artifact::{ArtifactKey, ArtifactPayload};
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
        // require expanded source view
        context
            .require(ArtifactKey::dir_expanded(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let bound = self
            .dir_bound(context, module, profile)
            .map_err(CompilerError::from)?;
        let imported = self
            .dir_imported(context, module, profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .dir_expanded(context, module, profile)
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
