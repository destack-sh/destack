use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirVerified};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};
use std::sync::Arc;

use crate::verify::{DropInsert, OwnershipCheck, VerifyState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the verified MIR marker of one module and target.
    pub(crate) fn collect_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));

        Ok(dependencies)
    }

    /// Build verified MIR marker for one module and target.
    pub(crate) fn provide_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let mut state = VerifyState::new(module, profile, target, context);
        let artifacts = self.artifact_reader(state.context.revision());

        // load provider inputs
        let lowered = artifacts
            .mir_lowered(state.module, state.profile, state.target)
            .map_err(CompilerError::from)?;
        let mut tree = lowered.tree.clone();

        OwnershipCheck::new(&mut tree, &mut state).run();
        if !state.has_errors() {
            DropInsert::new(&mut tree).run();
        }

        for diagnostic in state.take_diagnostics() {
            state.context.emit(diagnostic.as_ref())?;
        }

        Ok(ArtifactPayload::MirVerified(Arc::new(
            MirVerified::from_tree(tree),
        )))
    }
}
