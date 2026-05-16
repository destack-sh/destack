use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactPayload, DirExpanded};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build expanded DIR for one module.
    pub(crate) fn provide_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require imported dependency edges
        context
            .require(ArtifactKey::dir_imported(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let bound = self
            .dir_bound(context, module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper expansion
        let expanded = DirExpanded {
            patch: dir::Patch::new(&parsed.tree, "expand"),
            bindings: Arc::new(dir::BindingSegment::from_base(&bound.bindings)),
            dependencies: Arc::new(dir::DependencySegment::new()),
            types: Arc::new(dir::TypeSegment::from_base(&bound.types)),
            macros: dir::MacroTable::new(module),
            roots: bound.roots.clone(),
        };

        Ok(ArtifactPayload::DirExpanded(expanded))
    }
}
