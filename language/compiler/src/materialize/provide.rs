use destack_artifact::{ArtifactKey, ArtifactPayload, DirMaterialized};
use destack_dir::{BindingTable, CaptureTable, LayoutTable, Patch, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::materialize::MaterializeState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build materialized DIR for one module.
    pub(crate) fn provide_dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = MaterializeState::new(module, profile, context);
        state
            .context
            .require(ArtifactKey::dir_checked(state.module, state.profile))
            .map_err(CompilerError::from)?;
        let checked = self
            .dir_checked(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .dir_expanded(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;
        let declared = self
            .dir_declared(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::DirMaterialized(DirMaterialized {
            patch: Patch::new(&declared.tree, "materialize"),
            bindings: BindingTable::from_base(&expanded.bindings),
            types: TypeTable::from_base(&checked.types),
            captures: CaptureTable::new(),
            layouts: LayoutTable::new(state.module),
            roots: expanded.roots.clone(),
        }))
    }
}
