use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, EnvironmentBound,
};
use destack_core::{FxIndexMap, FxIndexSet};
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

    /// Return one nominal declaration's instance field keys in declaration order.
    pub(crate) fn instance_field_keys(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Vec<dir::StaticKey>, ProviderError> {
        self.read_declaration_tables(symbol.module_id, |_, definitions| {
            let definition = definitions.definition(symbol).ok_or_else(|| {
                ProviderError::internal(format!(
                    "nominal field owner {symbol:?} has no checked definition"
                ))
            })?;
            let fields = definition
                .instance_fields()
                .map(|field| field.key)
                .collect();

            Ok(fields)
        })
    }

    /// Return whether one declaration inherits any instance member other than a constructor.
    pub(crate) fn inherits_instance_members(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<bool, ProviderError> {
        let mut visited = FxIndexSet::default();
        let mut pending = vec![symbol];

        // traverse checked base declarations once
        while let Some(current) = pending.pop() {
            if !visited.insert(current) {
                continue;
            }
            let (has_instance_member, bases) =
                self.read_declaration_tables(current.module_id, |_, definitions| {
                    let definition = definitions.definition(current).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "nominal declaration {current:?} has no checked definition"
                        ))
                    })?;
                    let has_instance_member = current != symbol
                        && definition
                            .members_in(dir::MemberSpace::Instance)
                            .any(|member| {
                                !matches!(
                                    member,
                                    dir::DefinitionMember::Method(method)
                                        if method.role == Some(dir::FunctionRole::Constructor)
                                )
                            });
                    let bases = definition
                        .bases()
                        .into_iter()
                        .map(|heritage| heritage.ty)
                        .collect::<Vec<_>>();

                    Ok((has_instance_member, bases))
                })?;
            if has_instance_member {
                return Ok(true);
            }

            // resolve the next base declarations from their checked types
            for base in bases {
                let base = self.strip_form(base)?;
                let ty = self.get_type(base)?;
                let base = ty.symbol().ok_or_else(|| {
                    ProviderError::internal(format!(
                        "checked heritage type {base:?} is not nominal"
                    ))
                })?;
                pending.push(base);
            }
        }

        Ok(false)
    }

    /// Return the access represented by one checked memory type.
    pub(super) fn memory_access(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<dir::Access, ProviderError> {
        let ty = self.get_type(type_id)?;
        let dir::Type::Memory(dir::MemoryLiteral::Access(access)) = ty else {
            return Err(ProviderError::internal(format!(
                "checked memory access {type_id:?} has non-access type {ty:?}"
            )));
        };

        Ok(access)
    }

    /// Return whether one checked type is a nominal enum.
    pub(crate) fn is_enum_type(&self, type_id: dir::GlobalTypeId) -> Result<bool, ProviderError> {
        let type_id = self.strip_form(type_id)?;
        let Some(symbol) = self.get_type(type_id)?.symbol() else {
            return Ok(false);
        };

        self.read_declaration_tables(symbol.module_id, |_, definitions| {
            Ok(definitions.enum_definition(symbol).is_some())
        })
    }

    /// Return the payload carried by one checked borrowed type.
    pub(super) fn borrow_form(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::BorrowForm>, ProviderError> {
        // select one borrowed memory form
        let dir::Type::Form(dir::FormType {
            form: dir::Form::Borrowed(borrow),
            ..
        }) = self.get_type(type_id)?
        else {
            return Ok(None);
        };

        // read the solved borrow payload
        self.read_types(type_id.module_id, |types| {
            let borrow = types.borrow_form_maybe(borrow).ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked borrowed type {type_id:?} has no borrow payload"
                ))
            })?;

            Ok(Some(*borrow))
        })
    }

    /// Return the access carried by one checked borrowed type.
    pub(crate) fn borrow_access(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let Some(borrow) = self.borrow_form(type_id)? else {
            return Ok(None);
        };

        self.memory_access(borrow.access).map(Some)
    }

    /// Return the function signature behind one callable type.
    pub(super) fn callable_signature_type_id(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::GlobalTypeId>, ProviderError> {
        let type_id = self.strip_form(type_id)?;
        let signature = match self.get_type(type_id)? {
            dir::Type::FunctionSignature(_) => Some(type_id),
            dir::Type::Function(function) => Some(function.signature),
            dir::Type::FunctionPointer(function) => Some(function.signature),
            _ => None,
        };

        Ok(signature)
    }

    /// Return the checked parameters from one function signature.
    pub(super) fn signature_parameters(
        &self,
        signature: dir::GlobalTypeId,
    ) -> Result<Vec<dir::FunctionParameterType>, ProviderError> {
        let signature_type = self.signature_type(signature)?;

        self.read_types(signature.module_id, |types| {
            let parameters = types.parameters(signature_type.parameters).to_vec();

            Ok(parameters)
        })
    }

    /// Return the checked result type id from one callable signature.
    pub(super) fn signature_return_type_id(
        &self,
        signature: dir::GlobalTypeId,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let signature_type = self.signature_type(signature)?;

        signature_type.return_type.ok_or_else(|| {
            ProviderError::internal(format!(
                "checked callable signature {signature:?} has no return type"
            ))
        })
    }

    /// Return one checked function signature payload.
    fn signature_type(
        &self,
        signature: dir::GlobalTypeId,
    ) -> Result<dir::FunctionSignatureType, ProviderError> {
        let ty = self.get_type(signature)?;
        let dir::Type::FunctionSignature(signature_id) = ty else {
            return Err(ProviderError::internal(format!(
                "callable signature {signature:?} has non-signature type {ty:?}"
            )));
        };

        self.read_types(signature.module_id, |types| {
            types.signature_maybe(signature_id).copied().ok_or_else(|| {
                ProviderError::internal(format!(
                    "callable signature {signature:?} has no function signature payload"
                ))
            })
        })
    }

    /// Return whether one checked type includes an element selected by `predicate`.
    pub fn type_includes(
        &self,
        type_id: dir::GlobalTypeId,
        mut predicate: impl FnMut(&dir::Type) -> bool,
    ) -> Result<bool, ProviderError> {
        let ty = self.get_type(type_id)?;
        let dir::Type::Union(union) = ty else {
            return Ok(predicate(&ty));
        };

        self.read_types(type_id.module_id, |types| {
            for element in types.type_ids(union.elements) {
                let element = self.get_type(*element)?;
                if predicate(&element) {
                    return Ok(true);
                }
            }

            Ok(false)
        })
    }

    /// Return whether one checked type includes undefined.
    pub fn type_includes_undefined(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<bool, ProviderError> {
        self.type_includes(type_id, dir::Type::is_undefined)
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

    /// Return whether one declaration carries any checked decorator.
    pub(crate) fn has_decorators(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<bool, ProviderError> {
        // resolve the declaration that owns the selected symbol
        let declaration = self.read_declaration_tables(symbol.module_id, |bindings, _| {
            let binding = bindings.get_symbol_maybe(symbol.local_id).ok_or_else(|| {
                ProviderError::internal(format!(
                    "selected symbol {symbol:?} is absent from its binding table"
                ))
            })?;

            binding.declaration.ok_or_else(|| {
                ProviderError::internal(format!("selected symbol {symbol:?} has no declaration"))
            })
        })?;

        // inspect checked applications attached to the declaration
        self.read_decorators(symbol.module_id, |decorators| {
            let has_decorator = decorators
                .applications_for_owner(declaration)
                .next()
                .is_some();

            Ok(has_decorator)
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

    /// Read the decorator table that owns globally addressed declarations.
    fn read_decorators<T>(
        &self,
        module: ModuleId,
        read: impl FnOnce(&dir::DecoratorTable<'_>) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        // read the table already loaded for direct inspection
        if let Some(module) = self.modules.get(&module) {
            return read(&module.decorators);
        }

        // compose the foreign table from its checked DIR
        let elaborated = self
            .artifacts
            .read::<DirElaborated>((module, self.profile))?;
        let checked = self.artifacts.read::<DirChecked>((module, self.profile))?;
        let decorators = checked.decorator_table(&elaborated);

        read(&decorators)
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
