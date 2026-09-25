use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported, DirImported,
    DirMaterialized, DirParsed, DirResolved, DirView, IndexKind, ModuleIndex,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderError, Repository, Revision};
use destack_source::{File, ModuleId};

use super::Dir;

/// A borrowed DIR module.
#[derive(Debug, Clone, Copy)]
pub struct DirModule<'a> {
    /// The indexed DIR.
    pub dir: &'a Dir<'a>,
    /// The module id.
    pub id: ModuleId,
    /// The contributing source files in parsed order.
    pub files: &'a [Arc<File>],
    /// The stacked stages through materialization.
    pub stages: &'a DirView,
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
    /// The decorator table.
    pub decorators: &'a dir::DecoratorTable<'static>,
    /// The resolution table.
    pub resolutions: &'a dir::ResolutionTable<'static>,
    /// The decision table.
    pub decisions: &'a dir::DecisionTable<'static>,
    /// The generic table.
    pub generics: &'a dir::GenericTable<'static>,
    /// The definition table.
    pub definitions: &'a dir::DefinitionTable<'static>,
    /// The member selections.
    pub members: &'a dir::MemberTable<'static>,
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

/// Owned DIR storage for one module.
#[derive(Debug)]
pub(super) struct DirModuleStorage {
    /// The module id.
    pub(super) id: ModuleId,
    /// The contributing source files in parsed order.
    files: Box<[Arc<File>]>,
    /// The stacked stages through materialization.
    stages: DirView,
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
    /// The decorator table.
    pub(super) decorators: dir::DecoratorTable<'static>,
    /// The resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The decision table.
    decisions: dir::DecisionTable<'static>,
    /// The generic table.
    pub(super) generics: dir::GenericTable<'static>,
    /// The definition table.
    pub(super) definitions: dir::DefinitionTable<'static>,
    /// The member selections.
    pub(super) members: dir::MemberTable<'static>,
    /// The layout policies.
    pub(super) representations: dir::RepresentationTable<'static>,
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
    /// Create a borrowed DIR module.
    pub(super) fn new(dir: &'a Dir<'a>, storage: &'a DirModuleStorage) -> Self {
        Self {
            dir,
            id: storage.id,
            files: &storage.files,
            stages: &storage.stages,
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
            members: &storage.members,
            coercions: &storage.coercions,
            captures: &storage.captures,
            flows: &storage.flows,
            indexes: &storage.indexes,
            roots: &storage.roots,
            module_node: storage.module_node,
            namespace_scope: storage.namespace_scope,
        }
    }

    /// Return the reduced type of one local node.
    pub fn node_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.node_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return whether the value one node carries copies, by the verdict sema recorded.
    pub(crate) fn satisfies_copy(&self, ty: dir::GlobalTypeId) -> Result<bool, ProviderError> {
        Ok(self.dir.copies(ty)? == Some(true))
    }

    /// Return whether one node cannot complete normally.
    pub(crate) fn is_diverging(&self, node: dir::LocalNodeIdAny) -> Result<bool, ProviderError> {
        Ok(matches!(self.node_type(node)?, dir::Type::Never))
    }

    /// Return whether one node sits under a statically absent gate.
    pub fn is_statically_absent(&self, node: dir::LocalNodeIdAny) -> bool {
        // climb the tree, checking each decorated ancestor's committed presence
        let tree = &self.stages.parsed.tree;
        let mut current = Some(node);
        while let Some(decorated) = current {
            if self.statics.presence(decorated) == Some(dir::StaticPresence::Absent) {
                return true;
            }
            current = tree.get_parent(decorated.id);
        }

        false
    }

    /// Return the reduced type id of one local node.
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
    /// Load one module's DIR.
    pub(super) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        module: Arc<Module>,
        artifacts: &ArtifactReader<'_>,
        indexes: &[IndexKind],
    ) -> Result<Self, ProviderError> {
        let module_id = module.id;

        // read the DIR artifacts, the stages stacked once
        let reader = artifacts;
        let resolved = reader.read::<DirResolved>((module_id, profile))?;
        let view = DirView::materialized(
            reader.read::<DirParsed>(module_id)?,
            reader.read::<DirBound>((module_id, profile))?,
            reader.read::<DirImported>((module_id, profile))?,
            reader.read::<DirExpanded>((module_id, profile))?,
            resolved.clone(),
            reader.read::<DirDeclared>((module_id, profile))?,
            reader.read::<DirElaborated>((module_id, profile))?,
            reader.read::<DirChecked>((module_id, profile))?,
            reader.read::<DirMaterialized>((module_id, profile))?,
        );
        let parsed = &view.parsed;
        let exported = artifacts.read::<DirExported>((module_id, profile))?;

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

        // take the tables
        let bindings = view.bindings().clone();
        let modules = view.modules().clone();
        let types = view.types().clone();
        let statics = view.statics().clone();
        let decorators = view.decorators().clone();
        let resolutions = view.resolutions().clone();
        let decisions = view.decisions().clone();
        let generics = view.generics().clone();
        let definitions = view.definitions().clone();
        let members = view.members().clone();
        let representations = view.representations().clone();
        let coercions = view.coercions().clone();
        let captures = view.captures().clone();
        let flows = view.flows().clone();
        let roots = view.expanded.roots.clone();
        let module_node = view.bound.module_node;
        let namespace_scope = view.bound.namespace_scope;

        Ok(Self {
            id: module_id,
            files: files.into_boxed_slice(),
            stages: view,
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
            members,
            representations,
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
