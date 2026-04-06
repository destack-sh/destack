use destack_artifact::ArtifactKey;
use destack_dir::{NodeTree, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::analyze::common::TreeSymbolView;
use crate::{Compiler, CompilerContext, RequirementError};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Provide one tree-symbol view with exact artifact reads and local reuse.
    pub(crate) fn with_module_tree_symbol_view_or_local_for_artifact<R>(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(TreeSymbolView<'_>) -> R,
    ) -> Result<R, RequirementError> {
        if module_id == module.id {
            return Ok(handle(TreeSymbolView::new(
                context, module, profile, tree, symbols,
            )));
        }

        // remote reads require one exact committed artifact
        self.with_remote_dir_for_artifact(
            context,
            module_id,
            profile,
            artifact_key,
            |remote_module, tree, symbols, _| {
                handle(TreeSymbolView::new(
                    context,
                    remote_module,
                    profile,
                    tree,
                    symbols,
                ))
            },
        )
    }
}
