use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirMaterialized, MirLowered, MirVerified,
};
use destack_core::FxIndexSet;
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use crate::instantiate::InstantiateState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect the inputs that instantiate this module's bodies.
    pub(crate) fn collect_mir_instantiated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // require instance declarations, lowered bodies, and verification
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::dir_materialized(module, profile));
        dependencies.require_payload(ArtifactKey::mir_lowered(module, profile, target));
        dependencies.require_payload(ArtifactKey::mir_verified(module, profile, target));

        // read materialized instances to identify their template modules
        let artifacts = self.artifact_reader(context);
        let materialized = match artifacts.read::<DirMaterialized>((module, profile)) {
            Ok(materialized) => materialized,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // require the generic bodies of imported templates
        for template in Self::template_modules(module, &materialized) {
            dependencies.require_payload(ArtifactKey::mir_lowered(template, profile, target));
            dependencies.require(ArtifactKey::mir_verified(template, profile, target));
        }

        Ok(dependencies)
    }

    /// Instantiate this module's bodies from verified templates.
    pub(crate) fn provide_mir_instantiated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read instance declarations, lowered bodies, and verification
        let artifacts = self.artifact_reader(context);
        let materialized = artifacts
            .read::<DirMaterialized>((module, profile))
            .map_err(CompilerError::from)?;
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        let verified = artifacts
            .read::<MirVerified>((module, profile, target))
            .map_err(CompilerError::from)?;
        let mut state =
            InstantiateState::new(module, lowered, self.strings(), &artifacts, profile, target);

        // read foreign templates through the provider's tracked artifact reader
        for template in Self::template_modules(module, &materialized) {
            let lowered = artifacts
                .read::<MirLowered>((template, profile, target))
                .map_err(CompilerError::from)?;
            state.add_source(template, lowered);
        }

        // instantiate the requested function bodies
        state.instantiate()?;

        Ok(state.finish(&verified)?.into())
    }

    /// Return the other modules declaring the templates one module's instances apply.
    pub(crate) fn template_modules(
        module: ModuleId,
        materialized: &DirMaterialized,
    ) -> FxIndexSet<ModuleId> {
        materialized
            .generics
            .iter_instances()
            .map(|(_, instance)| instance.key.symbol.module_id)
            .filter(|template| *template != module)
            .collect()
    }
}
