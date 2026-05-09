use destack_artifact::{ArtifactKey, ArtifactPayload, DirExpanded};
use destack_dir::{BindingTable, DependencyTable, MacroTable, Patch, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::expand::ExpandState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Build expanded DIR for one module.
    pub(crate) fn provide_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ExpandState::new(module, profile, context);
        state
            .context
            .require(ArtifactKey::dir_imported(state.module, state.profile))?;
        let declared = self.dir_declared(state.context, state.module, state.profile)?;

        Ok(ArtifactPayload::DirExpanded(DirExpanded {
            patch: Patch::new(&declared.tree),
            bindings: BindingTable::from_base(&declared.bindings),
            dependencies: DependencyTable::new(state.module),
            types: TypeTable::from_base(&declared.types),
            macros: MacroTable::new(state.module),
            roots: declared.roots.clone(),
        }))
    }
}
