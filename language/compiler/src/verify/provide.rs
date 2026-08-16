use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirLowered, MirVerified,
};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::verify::Verifier;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for MIR verification.
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

    /// Verify one MIR module and target.
    pub(crate) fn provide_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        let mut verifier = Verifier::new(module, &lowered);

        verifier.verify();

        // emit every diagnostic and fail the verified artifact
        let mut errors = verifier.take_errors();
        let Some(error) = errors.pop() else {
            let retention = verifier.take_retention();

            return Ok(ArtifactPayload::MirVerified(Arc::new(MirVerified {
                retention,
            })));
        };
        for error in errors {
            context.emit(&error)?;
        }

        Err(CompilerError::Diagnostic(Box::new(error)))
    }
}
