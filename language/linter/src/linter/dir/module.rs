use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported, DirImported,
    DirParsed, DirResolved,
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
    /// The auto implementation table.
    pub auto: &'a dir::AutoTable<'static>,
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
    /// The auto implementation table.
    auto: dir::AutoTable<'static>,
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
            auto: &storage.auto,
            resolutions: &storage.resolutions,
            decisions: &storage.decisions,
            generics: &storage.generics,
            definitions: &storage.definitions,
            coercions: &storage.coercions,
            captures: &storage.captures,
            flows: &storage.flows,
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

    /// Return the reduced checked type id of one local node.
    pub fn node_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let node = node.into_global(self.id);
        self.types.get_node_type_id(node).ok_or_else(|| {
            ProviderError::internal(format!("checked DIR node {node:?} has no reduced type"))
        })
    }

    /// Return one checked node's type after its selected adjustments.
    pub fn adjusted_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.adjusted_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return one checked node's type id after its selected adjustments.
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
}

impl DirModuleStorage {
    /// Load one module's checked DIR.
    pub(super) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        module: Arc<Module>,
        artifacts: &ArtifactReader<'_>,
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
        let auto = elaborated.auto_table();
        let resolutions = checked.resolution_table(&declared, &elaborated);
        let decisions = checked.decision_table(&declared, &elaborated);
        let generics = checked.generic_table(&declared, &elaborated);
        let definitions = checked.definition_table(&elaborated);
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
            auto,
            resolutions,
            decisions,
            generics,
            definitions,
            coercions,
            captures,
            flows,
            roots,
            module_node,
            namespace_scope,
        })
    }
}
