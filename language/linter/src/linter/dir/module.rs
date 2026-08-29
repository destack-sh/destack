use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported, DirImported,
    DirParsed, DirResolved, IndexKind, ModuleIndex,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderError, Repository, Revision};
use destack_source::{File, ModuleId};

use super::Dir;

/// A borrowed checked DIR module.
#[derive(Debug, Clone, Copy)]
pub struct DirModule<'a> {
    /// The indexed checked DIR.
    pub dir: &'a Dir<'a>,
    /// The module id.
    pub id: ModuleId,
    /// The contributing source files in parsed order.
    pub files: &'a [Arc<File>],
    /// The parsed DIR artifact.
    pub parsed: &'a DirParsed,
    /// The expanded DIR artifact.
    pub expanded: &'a DirExpanded,
    /// The resolved import and source-reference artifact.
    pub resolved: &'a DirResolved,
    /// The resolved export artifact.
    pub exported: &'a DirExported,
    /// The binding table.
    pub bindings: &'a dir::BindingTable<'static>,
    /// The module dependency table.
    pub modules: &'a dir::ModuleTable<'static>,
    /// The type table.
    pub types: &'a dir::TypeTable<'static>,
    /// The static table.
    pub statics: &'a dir::StaticTable<'static>,
    /// The checked decorator table.
    pub decorators: &'a dir::DecoratorTable<'static>,
    /// The resolution table.
    pub resolutions: &'a dir::ResolutionTable<'static>,
    /// The decision table.
    pub decisions: &'a dir::DecisionTable<'static>,
    /// The generic table.
    pub generics: &'a dir::GenericTable<'static>,
    /// The definition table.
    pub definitions: &'a dir::DefinitionTable<'static>,
    /// The coercion table.
    pub coercions: &'a dir::CoercionTable<'static>,
    /// The capture table.
    pub captures: &'a dir::CaptureTable<'static>,
    /// The flow conclusion table.
    pub flows: &'a dir::FlowTable<'static>,
    /// The loaded module indexes by stable kind ordinal.
    indexes: &'a [Option<Arc<ModuleIndex>>; IndexKind::ALL.len()],
    /// The top-level expression roots.
    pub roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The stable module node.
    pub module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

/// Owned checked DIR storage for one module.
#[derive(Debug)]
pub(super) struct DirModuleStorage {
    /// The module id.
    pub(super) id: ModuleId,
    /// The contributing source files in parsed order.
    files: Box<[Arc<File>]>,
    /// The parsed DIR artifact.
    parsed: Arc<DirParsed>,
    /// The expanded DIR artifact.
    expanded: Arc<DirExpanded>,
    /// The resolved import and source-reference artifact.
    resolved: Arc<DirResolved>,
    /// The resolved export artifact.
    exported: Arc<DirExported>,
    /// The binding table.
    pub(super) bindings: dir::BindingTable<'static>,
    /// The module dependency table.
    modules: dir::ModuleTable<'static>,
    /// The type table.
    pub(super) types: dir::TypeTable<'static>,
    /// The static table.
    pub(super) statics: dir::StaticTable<'static>,
    /// The checked decorator table.
    pub(super) decorators: dir::DecoratorTable<'static>,
    /// The resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The decision table.
    decisions: dir::DecisionTable<'static>,
    /// The generic table.
    pub(super) generics: dir::GenericTable<'static>,
    /// The definition table.
    pub(super) definitions: dir::DefinitionTable<'static>,
    /// The coercion table.
    coercions: dir::CoercionTable<'static>,
    /// The capture table.
    captures: dir::CaptureTable<'static>,
    /// The flow conclusion table.
    flows: dir::FlowTable<'static>,
    /// The loaded module indexes by stable kind ordinal.
    indexes: [Option<Arc<ModuleIndex>>; IndexKind::ALL.len()],
    /// The top-level expression roots.
    roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The stable module node.
    module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
}

impl<'a> DirModule<'a> {
    /// Create a borrowed checked DIR module.
    pub(super) fn new(dir: &'a Dir<'a>, storage: &'a DirModuleStorage) -> Self {
        Self {
            dir,
            id: storage.id,
            files: &storage.files,
            parsed: &storage.parsed,
            expanded: &storage.expanded,
            resolved: &storage.resolved,
            exported: &storage.exported,
            bindings: &storage.bindings,
            modules: &storage.modules,
            types: &storage.types,
            statics: &storage.statics,
            decorators: &storage.decorators,
            resolutions: &storage.resolutions,
            decisions: &storage.decisions,
            generics: &storage.generics,
            definitions: &storage.definitions,
            coercions: &storage.coercions,
            captures: &storage.captures,
            flows: &storage.flows,
            indexes: &storage.indexes,
            roots: &storage.roots,
            module_node: storage.module_node,
            namespace_scope: storage.namespace_scope,
        }
    }

    /// Return the reduced checked type of one local node.
    pub fn node_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.node_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return whether the value one node carries copies by value.
    pub(crate) fn satisfies_copy(&self, ty: dir::GlobalTypeId) -> Result<bool, ProviderError> {
        let ty = self.dir.strip_form(ty)?;

        self.conforms(ty, dir::AutoInterface::Copy)
    }

    /// Return whether one checked type satisfies an auto interface through the committed tables.
    pub(crate) fn conforms(
        &self,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> Result<bool, ProviderError> {
        match self.dir.get_type(ty)? {
            // applied nominals read the committed declaration or instance conformances
            dir::Type::Application(application) => {
                let arguments = self.dir.read_types(ty.module_id, |types| {
                    Ok(types.type_ids(application.arguments).to_vec())
                })?;
                if arguments.is_empty() {
                    return self.definition_conforms(application.symbol, interface);
                }

                self.instance_conforms(application.symbol, &arguments, interface)
            }
            // declaration references read the committed declaration conformances
            dir::Type::Reference(reference) => {
                self.definition_conforms(reference.symbol, interface)
            }
            // parameters read the interface closure their bounds committed
            dir::Type::Parameter(parameter) => {
                let conformances = self.dir.read_generics(parameter.module_id, |generics| {
                    Ok(generics
                        .get_parameter(parameter.local_id)
                        .conformances
                        .clone())
                })?;

                Ok(conformances.contains(interface))
            }
            // scalars and singleton values copy by their machine representation
            dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Range(_)
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Never
            | dir::Type::Void => Ok(matches!(interface, dir::AutoInterface::Copy)),
            // unions satisfy component interfaces when every alternative does
            dir::Type::Union(union) => {
                let elements = self.dir.read_types(ty.module_id, |types| {
                    Ok(types.type_ids(union.elements).to_vec())
                })?;
                for element in elements {
                    if !self.conforms(element, interface)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            // object values ride managed handles, deeper interfaces decide property-wise
            dir::Type::Object(shape) => {
                if matches!(interface, dir::AutoInterface::Copy) {
                    return Ok(true);
                }
                let stores = self.dir.read_types(ty.module_id, |types| {
                    Ok(types
                        .properties(shape.properties)
                        .iter()
                        .map(|property| property.access.store())
                        .collect::<Vec<_>>())
                })?;
                for store in stores {
                    if !self.conforms(store, interface)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            // tuples satisfy component interfaces when every element does
            dir::Type::Tuple(tuple) => {
                let elements = self.dir.read_types(ty.module_id, |types| {
                    Ok(types
                        .elements(tuple.elements)
                        .iter()
                        .map(|element| element.ty)
                        .collect::<Vec<_>>())
                })?;
                for element in elements {
                    if !self.conforms(element, interface)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }

            _ => Ok(false),
        }
    }

    /// Return whether one declaration's committed conformances include an interface.
    fn definition_conforms(
        &self,
        symbol: dir::GlobalSymbolId,
        interface: dir::AutoInterface,
    ) -> Result<bool, ProviderError> {
        self.dir
            .read_declaration_tables(symbol.module_id, |_, definitions| {
                let conformances = definitions
                    .definition(symbol)
                    .and_then(|definition| definition.conformances().cloned());

                Ok(conformances.is_some_and(|conformances| conformances.contains(interface)))
            })
    }

    /// Return whether one materialized nominal instance satisfies an auto interface.
    fn instance_conforms(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        interface: dir::AutoInterface,
    ) -> Result<bool, ProviderError> {
        // find the materialized instance whose key matches the applied arguments
        for (_, instance) in self.generics.iter_instances() {
            if instance.key.symbol != symbol {
                continue;
            }
            let bound: Vec<_> =
                dir::GenericArgumentBinding::values(&instance.key.arguments).collect();
            if bound.len() != arguments.len() {
                continue;
            }
            let mut is_match = true;
            for (argument, bound) in arguments.iter().zip(&bound) {
                is_match &= self.dir.types_match(*argument, *bound)?;
            }
            if is_match {
                return Ok(instance.conformances.contains(interface));
            }
        }

        Ok(false)
    }

    /// Return whether one node cannot complete normally.
    pub(crate) fn is_diverging(&self, node: dir::LocalNodeIdAny) -> Result<bool, ProviderError> {
        Ok(matches!(self.node_type(node)?, dir::Type::Never))
    }

    /// Return whether one node sits under a statically absent gate.
    pub fn is_statically_absent(&self, node: dir::LocalNodeIdAny) -> bool {
        // climb the tree, checking each decorated ancestor's committed presence
        let tree = &self.parsed.tree;
        let mut current = Some(node);
        while let Some(decorated) = current {
            if self.statics.presence(decorated) == Some(dir::StaticPresence::Absent) {
                return true;
            }
            current = tree.get_parent(decorated.id);
        }

        false
    }

    /// Return the reduced checked type id of one local node.
    pub fn node_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let node = node.into_global(self.id);
        self.types
            .get_node_type_id(node)
            .ok_or_else(|| ProviderError::internal(format!("node {node:?} has no reduced type")))
    }

    /// Return one node's type after its selected adjustments.
    pub fn adjusted_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.adjusted_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return whether one node passes its value on unchanged.
    pub(crate) fn is_unadjusted(&self, node: dir::LocalNodeIdAny) -> bool {
        let Some(coercion) = self.coercions.coercion(node.into_global(self.id)) else {
            return true;
        };

        // widening settles a fresh numeric literal at its representation and leaves the value alone
        coercion
            .adjustments
            .iter()
            .all(|adjustment| matches!(adjustment, dir::CoercionAdjustment::Materialize { .. }))
    }

    /// Return one node's type id after its selected adjustments.
    pub fn adjusted_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let global = node.into_global(self.id);
        let type_id = match self.coercions.coercion(global) {
            Some(coercion) => coercion.target(),
            None => self.node_type_id(node)?,
        };

        Ok(type_id)
    }

    /// Return one loaded module index.
    fn index(&self, kind: IndexKind) -> Result<&ModuleIndex, ProviderError> {
        let Some(index) = self.indexes[kind.ordinal()].as_deref() else {
            return Err(ProviderError::internal(format!(
                "DIR module {:?} was loaded without its {kind:?} index",
                self.id,
            )));
        };

        Ok(index)
    }

    /// Return the code fingerprint index.
    fn code(&self) -> Result<&dir::CodeIndex, ProviderError> {
        match self.index(IndexKind::Code)? {
            ModuleIndex::Code(index) => Ok(index),
            index => Err(ProviderError::internal(format!(
                "DIR module {:?} code index slot contains {:?}",
                self.id,
                index.kind()
            ))),
        }
    }

    /// Return one visible node's name-insensitive structural fingerprint.
    pub(crate) fn code_fingerprint(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::CodeFingerprint, ProviderError> {
        self.code()?.fingerprint(node.id).ok_or_else(|| {
            ProviderError::internal(format!("visible node {node:?} has no code fingerprint"))
        })
    }
}

impl DirModuleStorage {
    /// Load one module's checked DIR.
    pub(super) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        module: Arc<Module>,
        artifacts: &ArtifactReader<'_>,
        indexes: &[IndexKind],
    ) -> Result<Self, ProviderError> {
        let module_id = module.id;

        // read the DIR artifacts
        let parsed = artifacts.read::<DirParsed>(module_id)?;
        let bound = artifacts.read::<DirBound>((module_id, profile))?;
        let imported = artifacts.read::<DirImported>((module_id, profile))?;
        let resolved = artifacts.read::<DirResolved>((module_id, profile))?;
        let expanded = artifacts.read::<DirExpanded>((module_id, profile))?;
        let exported = artifacts.read::<DirExported>((module_id, profile))?;
        let checked = artifacts.read::<DirChecked>((module_id, profile))?;
        let declared = artifacts.read::<DirDeclared>((module_id, profile))?;
        let elaborated = artifacts.read::<DirElaborated>((module_id, profile))?;

        // load each index explicitly requested by an active lint
        let mut loaded_indexes = std::array::from_fn(|_| None);
        for kind in indexes.iter().copied() {
            let index = artifacts.read::<ModuleIndex>((module_id, profile, kind))?;
            if index.kind() != kind {
                return Err(ProviderError::internal(format!(
                    "module {module_id:?} requested {kind:?} index but loaded {:?}",
                    index.kind()
                )));
            }
            loaded_indexes[kind.ordinal()] = Some(index);
        }

        let expected_files = module
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        let parsed_files = parsed
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        if parsed_files != expected_files {
            return Err(ProviderError::Internal {
                message: format!(
                    "lint module {module_id:?} parsed files {parsed_files:?} do not match repository files {expected_files:?}"
                ),
            });
        }

        // load source files
        let mut files = Vec::with_capacity(parsed_files.len());
        for file_id in parsed_files {
            let file = repository
                .file(revision, file_id)
                .map_err(|error| ProviderError::Internal {
                    message: error.to_string(),
                })?
                .ok_or_else(|| ProviderError::Internal {
                    message: format!("missing lint file {file_id:?}"),
                })?;
            files.push(file);
        }

        // compose the checked tables
        let bindings = checked.binding_table(&bound, &expanded, &declared, &elaborated);
        let modules = expanded.module_table(&imported);
        let types = checked.type_table(&bound, &expanded, &declared, &elaborated);
        let statics = checked.static_table(&bound, &expanded, &declared, &elaborated);
        let decorators = checked.decorator_table(&elaborated);
        let resolutions = checked.resolution_table(&declared, &elaborated);
        let decisions = checked.decision_table(&declared, &elaborated);
        let generics = checked.generic_table(&declared, &elaborated);
        let definitions = checked.definition_table(&declared, &elaborated);
        let coercions = checked.coercion_table();
        let captures = checked.capture_table();
        let flows = checked.flow_table(&declared, &elaborated);
        let roots = expanded.roots.clone();
        let module_node = bound.module_node;
        let namespace_scope = bound.namespace_scope;

        Ok(Self {
            id: module_id,
            files: files.into_boxed_slice(),
            parsed,
            expanded,
            resolved,
            exported,
            bindings,
            modules,
            types,
            statics,
            decorators,
            resolutions,
            decisions,
            generics,
            definitions,
            coercions,
            captures,
            flows,
            indexes: loaded_indexes,
            roots,
            module_node,
            namespace_scope,
        })
    }
}
