use std::sync::Arc;

use tspp_artifact::EnvironmentBound;
use tspp_core::{FxIndexMap, StringPool};
use tspp_repository::{ArtifactReader, ProfileId, ProviderError, Repository, Revision};
use tspp_source::{ModuleId, TargetId};

use super::MirModule;

/// Verified MIR indexed by module id.
#[derive(Debug)]
pub struct Mir<'a> {
    /// The active profile.
    pub profile: ProfileId,
    /// The active target.
    pub target: TargetId,
    /// The repository string pool.
    pub strings: Arc<StringPool>,
    /// The loaded verified modules.
    modules: FxIndexMap<ModuleId, MirModule<'a>>,
}

impl<'a> Mir<'a> {
    /// Return one loaded verified module.
    pub fn module(&self, module: ModuleId) -> Result<&MirModule<'a>, ProviderError> {
        self.modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("MIR module {module:?} is not loaded"),
            })
    }

    /// Return one loaded verified module for analysis.
    pub fn module_mut(&mut self, module: ModuleId) -> Result<&mut MirModule<'a>, ProviderError> {
        self.modules
            .get_mut(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("MIR module {module:?} is not loaded"),
            })
    }

    /// Iterate the loaded verified modules.
    pub fn modules(&self) -> impl Iterator<Item = &MirModule<'a>> {
        self.modules.values()
    }

    /// Iterate the loaded verified modules for analysis.
    pub fn modules_mut(&mut self) -> impl Iterator<Item = &mut MirModule<'a>> {
        self.modules.values_mut()
    }

    /// Load verified MIR for selected modules beside their DIR.
    pub(crate) fn load(
        repository: &'a Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'a>,
        profile: ProfileId,
        target: TargetId,
        modules: &[ModuleId],
        strings: Arc<StringPool>,
    ) -> Result<Self, ProviderError> {
        let environment = artifacts.read::<EnvironmentBound>(profile)?;
        let mut loaded = FxIndexMap::default();
        loaded.reserve(modules.len());

        // load every selected module
        for module in modules.iter().copied() {
            let loaded_module = MirModule::load(
                repository,
                revision,
                artifacts,
                profile,
                target,
                module,
                strings.clone(),
                environment.clone(),
            )?;
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
