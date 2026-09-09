use std::sync::Arc;

use destack_artifact::{DirMaterialized, MirLowered};
use destack_core::FxIndexSet;
use destack_repository::{ArtifactReader, ProfileId};
use destack_source::{ModuleId, TargetId};

use crate::instantiate::{InstantiateState, Instantiated};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Give every specialization one module's MIR demands its body.
    pub(crate) fn instantiate_module(
        &self,
        module: ModuleId,
        materialized: &DirMaterialized,
        lowered: Arc<MirLowered>,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
    ) -> CompilerResult<Instantiated> {
        let mut state =
            InstantiateState::new(module, lowered, self.strings(), artifacts, profile, target);
        for template in Self::template_modules(module, materialized) {
            let lowered = artifacts
                .read::<MirLowered>((template, profile, target))
                .map_err(CompilerError::from)?;
            state.add_source(template, lowered);
        }
        state.instantiate()?;

        state.finish()
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
