use std::sync::Arc;

use destack_artifact::EnvironmentBound;
use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::{DirModule, DirModuleStorage};

/// Checked DIR available to one lint run.
#[derive(Debug)]
pub struct Dir<'a> {
    /// The active profile.
    pub profile: ProfileId,
    /// The global language environment.
    pub environment: Arc<EnvironmentBound>,
    /// The repository string pool.
    pub strings: Arc<dir::StringPool>,
    /// The repository that owns the inspected modules.
    repository: &'a Repository,
    /// The repository revision.
    revision: Revision,
    /// The artifact reader for globally owned DIR values.
    artifacts: ArtifactReader<'a>,
    /// The modules inspected by this lint artifact.
    modules: FxIndexMap<ModuleId, DirModuleStorage>,
}

impl<'a> Dir<'a> {
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
        self.read_module(type_id.module_id, |module| {
            module
                .types
                .get_type_maybe(type_id.local_id)
                .ok_or_else(|| ProviderError::Internal {
                    message: format!("DIR type {type_id:?} is not present in its owning module"),
                })
        })
    }

    /// Return the borrow form carried by one checked type.
    pub fn get_borrow(&self, type_id: dir::GlobalTypeId) -> Result<dir::BorrowForm, ProviderError> {
        self.read_module(type_id.module_id, |module| {
            let ty = module
                .types
                .get_type_maybe(type_id.local_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "borrow target {type_id:?} is absent from its owning type table"
                    ))
                })?;
            let dir::Type::Form(form) = ty else {
                return Err(ProviderError::internal(format!(
                    "borrow target {type_id:?} is not a form type"
                )));
            };
            let dir::Form::Borrowed(borrow_id) = form.form else {
                return Err(ProviderError::internal(format!(
                    "borrow target {type_id:?} is not borrowed"
                )));
            };

            module
                .types
                .borrow_form_maybe(borrow_id)
                .copied()
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "borrow target {type_id:?} has no borrow form {borrow_id:?}"
                    ))
                })
        })
    }

    /// Return the access carried by one checked memory singleton.
    pub fn get_access(&self, type_id: dir::GlobalTypeId) -> Result<dir::Access, ProviderError> {
        let ty = self.get_type(type_id)?;
        let dir::Type::Memory(dir::MemoryLiteral::Access(access)) = ty else {
            return Err(ProviderError::internal(format!(
                "DIR type {type_id:?} is not an access singleton"
            )));
        };

        Ok(access)
    }

    /// Return one checked static value by global id.
    pub fn get_static(
        &self,
        static_id: dir::GlobalStaticId,
    ) -> Result<dir::StaticTerm, ProviderError> {
        self.read_module(static_id.module_id, |module| {
            module
                .statics
                .get_static_maybe(static_id.local_id)
                .cloned()
                .ok_or_else(|| ProviderError::Internal {
                    message: format!(
                        "DIR static value {static_id:?} is not present in its owning module"
                    ),
                })
        })
    }

    /// Return the checked static value selected by one symbol.
    pub fn symbol_static(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::StaticTerm>, ProviderError> {
        self.read_module(symbol.module_id, |module| {
            let value = module
                .statics
                .get_symbol_static_id(symbol)
                .and_then(|static_id| module.statics.get_static_maybe(static_id.local_id))
                .cloned();

            Ok(value)
        })
    }

    /// Load checked DIR for selected modules.
    pub(crate) fn load(
        repository: &'a Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'a>,
        profile: ProfileId,
        environment: Arc<EnvironmentBound>,
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
            repository,
            revision,
            artifacts: artifacts.clone(),
            modules: loaded,
        })
    }

    /// Read one module that owns globally addressed DIR values.
    pub(super) fn read_module<T>(
        &self,
        module: ModuleId,
        read: impl FnOnce(&DirModuleStorage) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        // read modules already loaded for direct inspection
        if let Some(module) = self.modules.get(&module) {
            return read(module);
        }

        // select the foreign code module
        let repository_module = self
            .repository
            .module(self.revision, module)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| ProviderError::internal(format!("missing DIR module {module:?}")))?;
        if !repository_module.is_code() {
            return Err(ProviderError::internal(format!(
                "globally addressed DIR value belongs to non-code module {module:?}"
            )));
        }

        // load its checked DIR through recorded artifact reads
        let module = DirModuleStorage::load(
            self.repository,
            self.revision,
            self.profile,
            repository_module,
            &self.artifacts,
        )?;

        read(&module)
    }
}
