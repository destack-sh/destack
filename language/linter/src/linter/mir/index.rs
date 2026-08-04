use std::sync::Arc;

use destack_core::{FxIndexMap, StringPool};
use destack_repository::{ArtifactReader, ProfileId, ProviderError};
use destack_source::{ModuleId, TargetId};

use super::MirModule;

/// Verified MIR indexed by module id.
#[derive(Debug)]
pub struct Mir {
    /// The active profile.
    pub profile: ProfileId,
    /// The active target.
    pub target: TargetId,
    /// The repository string pool.
    pub strings: Arc<StringPool>,
    /// The loaded verified modules.
    modules: FxIndexMap<ModuleId, MirModule>,
}

impl Mir {
    /// Return one loaded verified module.
    pub fn module(&self, module: ModuleId) -> Result<&MirModule, ProviderError> {
        self.modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("MIR module {module:?} is not loaded"),
            })
    }

    /// Iterate the loaded verified modules.
    pub fn modules(&self) -> impl Iterator<Item = &MirModule> {
        self.modules.values()
    }

    /// Load verified MIR for selected modules.
    pub(crate) fn load(
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
        modules: &[ModuleId],
        strings: Arc<StringPool>,
    ) -> Result<Self, ProviderError> {
        let mut loaded = FxIndexMap::default();
        loaded.reserve(modules.len());

        // load every selected module
        for module in modules.iter().copied() {
            let loaded_module =
                MirModule::load(artifacts, profile, target, module, strings.clone())?;
            loaded.insert(module, loaded_module);
        }

        Ok(Self {
            profile,
            target,
            strings,
            modules: loaded,
        })
    }
}
