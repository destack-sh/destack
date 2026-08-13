use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, EnvironmentBound,
};
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
        self.read_types(type_id.module_id, |types| {
            types
                .get_type_maybe(type_id.local_id)
                .ok_or_else(|| ProviderError::Internal {
                    message: format!("DIR type {type_id:?} is not present in its owning module"),
                })
        })
    }

    /// Strip placement forms from one checked type id.
    pub fn strip_form(
        &self,
        mut type_id: dir::GlobalTypeId,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        // follow placement forms to their represented value
        while let dir::Type::Form(form) = self.get_type(type_id)? {
            type_id = form.value;
        }

        Ok(type_id)
    }

    /// Return the checked result type id from one callable signature.
    pub(super) fn signature_return_type_id(
        &self,
        signature: dir::GlobalTypeId,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let ty = self.get_type(signature)?;
        let dir::Type::FunctionSignature(signature_id) = ty else {
            return Err(ProviderError::internal(format!(
                "callable signature {signature:?} has non-signature type {ty:?}"
            )));
        };

        self.read_types(signature.module_id, |types| {
            let signature = types.signature_maybe(signature_id).ok_or_else(|| {
                ProviderError::internal(format!(
                    "callable signature {signature:?} has no function signature payload"
                ))
            })?;

            signature.return_type.ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked callable signature {signature:?} has no return type"
                ))
            })
        })
    }

    /// Return whether one checked type includes undefined.
    pub fn type_includes_undefined(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<bool, ProviderError> {
        let ty = self.get_type(type_id)?;
        let dir::Type::Union(union) = ty else {
            return Ok(ty.is_undefined());
        };

        self.read_types(type_id.module_id, |types| {
            for element in types.type_ids(union.elements) {
                if self.get_type(*element)?.is_undefined() {
                    return Ok(true);
                }
            }

            Ok(false)
        })
    }

    /// Return one checked static value by global id.
    pub fn get_static(
        &self,
        static_id: dir::GlobalStaticId,
    ) -> Result<dir::StaticTerm, ProviderError> {
        self.read_statics(static_id.module_id, |statics| {
            statics
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
        self.read_statics(symbol.module_id, |statics| {
            let value = statics
                .get_symbol_static_id(symbol)
                .and_then(|static_id| statics.get_static_maybe(static_id.local_id))
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
            artifacts: artifacts.clone(),
            modules: loaded,
        })
    }

    /// Read the type table that owns globally addressed DIR types.
    fn read_types<T>(
        &self,
        module: ModuleId,
        read: impl FnOnce(&dir::TypeTable<'_>) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        // read the table already loaded for direct inspection
        if let Some(module) = self.modules.get(&module) {
            return read(&module.types);
        }

        // compose the foreign table from its checked DIR
        let bound = self.artifacts.read::<DirBound>((module, self.profile))?;
        let expanded = self.artifacts.read::<DirExpanded>((module, self.profile))?;
        let declared = self.artifacts.read::<DirDeclared>((module, self.profile))?;
        let elaborated = self
            .artifacts
            .read::<DirElaborated>((module, self.profile))?;
        let checked = self.artifacts.read::<DirChecked>((module, self.profile))?;
        let types = checked.type_table(&bound, &expanded, &declared, &elaborated);

        read(&types)
    }

    /// Read the static table that owns globally addressed DIR values.
    fn read_statics<T>(
        &self,
        module: ModuleId,
        read: impl FnOnce(&dir::StaticTable<'_>) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        // read the table already loaded for direct inspection
        if let Some(module) = self.modules.get(&module) {
            return read(&module.statics);
        }

        // compose the foreign table from its checked DIR
        let bound = self.artifacts.read::<DirBound>((module, self.profile))?;
        let expanded = self.artifacts.read::<DirExpanded>((module, self.profile))?;
        let declared = self.artifacts.read::<DirDeclared>((module, self.profile))?;
        let elaborated = self
            .artifacts
            .read::<DirElaborated>((module, self.profile))?;
        let checked = self.artifacts.read::<DirChecked>((module, self.profile))?;
        let statics = checked.static_table(&bound, &expanded, &declared, &elaborated);

        read(&statics)
    }

    /// Read binding and definition tables for one globally addressed declaration.
    pub(super) fn read_declaration_tables<T>(
        &self,
        module: ModuleId,
        read: impl FnOnce(&dir::BindingTable<'_>, &dir::DefinitionTable<'_>) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        // read the tables already loaded for direct inspection
        if let Some(module) = self.modules.get(&module) {
            return read(&module.bindings, &module.definitions);
        }

        // compose the foreign tables from their checked DIR
        let bound = self.artifacts.read::<DirBound>((module, self.profile))?;
        let expanded = self.artifacts.read::<DirExpanded>((module, self.profile))?;
        let declared = self.artifacts.read::<DirDeclared>((module, self.profile))?;
        let elaborated = self
            .artifacts
            .read::<DirElaborated>((module, self.profile))?;
        let checked = self.artifacts.read::<DirChecked>((module, self.profile))?;
        let bindings = checked.binding_table(&bound, &expanded, &declared, &elaborated);
        let definitions = checked.definition_table(&elaborated);

        read(&bindings, &definitions)
    }
}
