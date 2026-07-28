use std::sync::Arc;

use destack_core::{FxIndexMap, StringPool};
use destack_repository::{ArtifactReader, ProfileId, ProviderError};
use destack_source::{ModuleId, TargetId};

use super::{MirModule, MirModuleStorage};

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
    modules: FxIndexMap<ModuleId, MirModuleStorage>,
}

impl Mir {
    /// Return one loaded verified module.
    pub fn module(&self, module: ModuleId) -> Result<MirModule<'_>, ProviderError> {
        let storage = self
            .modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("MIR module {module:?} is not loaded"),
            })?;

        Ok(MirModule::new(self, storage))
    }

    /// Iterate the loaded verified modules.
    pub fn modules(&self) -> impl Iterator<Item = MirModule<'_>> {
        self.modules
            .values()
            .map(|storage| MirModule::new(self, storage))
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
            let storage = MirModuleStorage::load(artifacts, profile, target, module)?;
            loaded.insert(module, storage);
        }

        Ok(Self {
            profile,
            target,
            strings,
            modules: loaded,
        })
    }
}
