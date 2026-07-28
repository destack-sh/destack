use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::{DirModule, DirModuleStorage};

/// Checked DIR indexed by module id.
#[derive(Debug)]
pub struct Dir {
    /// The active profile.
    pub profile: ProfileId,
    /// The global language environment.
    pub environment: Arc<GlobalEnvironment>,
    /// The repository string pool.
    pub strings: Arc<dir::StringPool>,
    /// The loaded checked modules.
    modules: FxIndexMap<ModuleId, DirModuleStorage>,
}

impl Dir {
    /// Return one loaded checked module.
    pub fn module(&self, module: ModuleId) -> Result<DirModule<'_>, ProviderError> {
        let storage = self
            .modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("DIR module {module:?} is not loaded"),
            })?;

        Ok(DirModule::new(self, storage))
    }

    /// Iterate the loaded checked modules.
    pub fn modules(&self) -> impl Iterator<Item = DirModule<'_>> {
        self.modules
            .values()
            .map(|storage| DirModule::new(self, storage))
    }

    /// Return one checked type by global id.
    pub fn get_type(&self, type_id: dir::GlobalTypeId) -> Result<dir::Type, ProviderError> {
        let module = self.module_storage(type_id.module_id)?;

        module
            .types
            .get_type_maybe(type_id.local_id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("DIR type {type_id:?} is not present in its owning module"),
            })
    }

    /// Return one checked static value by global id.
    pub fn get_static(
        &self,
        static_id: dir::GlobalStaticId,
    ) -> Result<&dir::StaticTerm, ProviderError> {
        let module = self.module_storage(static_id.module_id)?;

        module
            .statics
            .get_static_maybe(static_id.local_id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR static value {static_id:?} is not present in its owning module"
                ),
            })
    }

    /// Load checked DIR for selected modules.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        environment: Arc<GlobalEnvironment>,
        modules: &[ModuleId],
    ) -> Result<Self, ProviderError> {
        let mut loaded = FxIndexMap::default();
        loaded.reserve(modules.len());

        // load every selected code module
        for module in modules.iter().copied() {
            let repository_module = repository
                .module(revision, module)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| ProviderError::internal(format!("missing DIR module {module:?}")))?;
            if !repository_module.is_code() {
                continue;
            }

            let module = DirModuleStorage::load(
                repository,
                revision,
                profile,
                repository_module,
                artifacts,
            )?;
            loaded.insert(module.id, module);
        }

        Ok(Self {
            profile,
            environment,
            strings: repository.string_pool().clone(),
            modules: loaded,
        })
    }

    /// Return loaded storage for one checked module.
    pub(super) fn module_storage(
        &self,
        module: ModuleId,
    ) -> Result<&DirModuleStorage, ProviderError> {
        self.modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("DIR module {module:?} is not loaded"),
            })
    }
}
