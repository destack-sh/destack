use std::slice;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, DirBound, DirCheckedModule, DirExpanded, DirExported, DirImported, DirParsed,
    DirParsedFile, DirResolved,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderResult, Repository, Revision};
use destack_source::{File, FileId, ModuleId, ProfileId, SourceIndex, Span};

use crate::{Module, QueryError, QueryResult};

/// Query context anchored to one module profile.
#[derive(Debug)]
pub struct ModuleQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The parsed module DIR.
    parsed: Arc<DirParsed>,
    /// The expanded module DIR.
    expanded: Arc<DirExpanded>,
    /// The exported module DIR.
    exported: Arc<DirExported>,
    /// The resolved import and source-reference DIR.
    resolved: Arc<DirResolved>,
    /// The checked binding table.
    bindings: dir::BindingTable<'static>,
    /// The visible module table.
    modules: dir::ModuleTable<'static>,
    /// The checked type table.
    types: dir::TypeTable<'static>,
    /// The checked decorator table.
    decorators: dir::DecoratorTable<'static>,
    /// The checked generic table.
    generics: dir::GenericTable<'static>,
    /// The checked definition table.
    definitions: dir::DefinitionTable<'static>,
    /// The checked resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
    /// Shared repository strings.
    strings: &'a StringPool,
    /// The revision used for this context.
    revision: Revision,
    /// The profile used for this context.
    profile_id: ProfileId,
    /// The module id.
    module_id: ModuleId,
}

/// Artifact payloads required to build one module query context.
struct ModuleArtifacts {
    /// The parsed module artifact.
    parsed: Arc<DirParsed>,
    /// The bound module artifact.
    bound: Arc<DirBound>,
    /// The imported module artifact.
    imported: Arc<DirImported>,
    /// The expanded module artifact.
    expanded: Arc<DirExpanded>,
    /// The exported module artifact.
    exported: Arc<DirExported>,
    /// The resolved import and source-reference artifact.
    resolved: Arc<DirResolved>,
    /// The checked module artifact.
    checked: Arc<DirCheckedModule>,
}

impl ModuleArtifacts {
    /// Read one coherent artifact set from a revision-bound reader.
    fn read(
        reader: &ArtifactReader<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<Self> {
        Ok(Self {
            parsed: reader.dir_parsed(module_id)?,
            bound: reader.dir_bound(module_id, profile_id)?,
            imported: reader.dir_imported(module_id, profile_id)?,
            expanded: reader.dir_expanded(module_id, profile_id)?,
            exported: reader.dir_exported(module_id, profile_id)?,
            resolved: reader.dir_resolved(module_id, profile_id)?,
            checked: reader.dir_checked(module_id, profile_id)?,
        })
    }
}

impl<'a> ModuleQueryContext<'a> {
    /// Build one module context from loaded artifacts.
    fn from_artifacts(
        repository: &'a Repository,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifacts: ModuleArtifacts,
    ) -> Self {
        // compose imported and checked table views
        let bindings = artifacts
            .checked
            .binding_table(&artifacts.bound, &artifacts.expanded);
        let modules = artifacts.expanded.module_table(&artifacts.imported);
        let namespace_scope = artifacts.bound.namespace_scope;

        // compose remaining checked table views
        let types = artifacts
            .checked
            .type_table(&artifacts.bound, &artifacts.expanded);
        let decorators = artifacts.checked.decorator_table();
        let generics = artifacts.checked.generic_table();
        let definitions = artifacts.checked.definition_table();
        let resolutions = artifacts.checked.resolution_table();

        Self {
            repository,
            parsed: artifacts.parsed,
            expanded: artifacts.expanded,
            exported: artifacts.exported,
            resolved: artifacts.resolved,
            bindings,
            modules,
            types,
            decorators,
            generics,
            definitions,
            resolutions,
            namespace_scope,
            strings: repository.string_pool().as_ref(),
            revision,
            profile_id,
            module_id,
        }
    }

    /// Return every artifact key read by one module query context.
    pub fn artifact_keys(module_id: ModuleId, profile_id: ProfileId) -> [ArtifactKey; 7] {
        [
            ArtifactKey::dir_parsed(module_id),
            ArtifactKey::dir_bound(module_id, profile_id),
            ArtifactKey::dir_imported(module_id, profile_id),
            ArtifactKey::dir_expanded(module_id, profile_id),
            ArtifactKey::dir_exported(module_id, profile_id),
            ArtifactKey::dir_resolved(module_id, profile_id),
            ArtifactKey::dir_checked(module_id, profile_id),
        ]
    }

    /// Read one module query context from its required artifact set.
    pub fn new(
        repository: &'a Repository,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<Self> {
        // read exact dependency-backed artifact payloads
        let reader = ArtifactReader::new(repository, revision);
        let artifacts = ModuleArtifacts::read(&reader, module_id, profile_id)?;

        Ok(Self::from_artifacts(
            repository, revision, module_id, profile_id, artifacts,
        ))
    }

    /// Return the repository for this module context.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the profile id for this module context.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the revision for this module context.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the module id for this module context.
    pub fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return one source file.
    pub(crate) fn read_file(&self, file_id: FileId) -> QueryResult<Arc<File>> {
        let file = self.repository().file(self.revision(), file_id)?;
        let file = file.ok_or(QueryError::missing(format!("source file: {file_id:?}")))?;

        Ok(file)
    }

    /// Return the exact authored text for one source span.
    pub(crate) fn source_text(&self, span: Span) -> QueryResult<String> {
        let file = self.read_file(span.file)?;
        let text = file
            .get_span_str(span)
            .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;

        Ok(text.to_string())
    }

    /// Return the queried module.
    pub fn module(&self) -> Module {
        Module {
            module_id: self.module_id,
            profile_id: self.profile_id,
        }
    }

    /// Return the parsed DIR source index.
    pub(crate) fn source_index(&self) -> &SourceIndex {
        &self.parsed.tree.source_index
    }

    /// Iterate semantic tokens for one source file.
    pub(crate) fn tokens(
        &self,
        file_id: FileId,
    ) -> QueryResult<impl Iterator<Item = dir::TokenSpan> + '_> {
        let file = self.parsed_file(file_id)?;

        Ok(file.iter_token_spans())
    }

    /// Return source comments for one source file.
    pub(crate) fn comments(&self, file_id: FileId) -> QueryResult<&[dir::Comment]> {
        let file = self.parsed_file(file_id)?;

        Ok(&file.comments)
    }

    /// Return parser output for one source file in this module.
    fn parsed_file(&self, file_id: FileId) -> QueryResult<&DirParsedFile> {
        let file = self
            .parsed
            .file(file_id)
            .ok_or(QueryError::missing(format!("parsed file: {file_id:?}")))?;

        Ok(file)
    }

    /// Return the authored roots for one source file.
    pub(crate) fn file_roots(
        &self,
        file_id: FileId,
    ) -> QueryResult<&[dir::LocalNodeId<dir::Expression>]> {
        let file = self.parsed_file(file_id)?;

        Ok(&file.roots)
    }

    /// Return the visible DIR tree view.
    pub(crate) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, slice::from_ref(&self.expanded.patch))
    }

    /// Return the resolved import and source-reference DIR.
    pub(crate) fn resolved(&self) -> &DirResolved {
        self.resolved.as_ref()
    }

    /// Return the DIR symbol table.
    pub(crate) fn symbols(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the symbol declared by one local node when bound.
    pub(crate) fn node_symbol(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalSymbolId> {
        let declaration = node_id.into_global(self.module_id);

        self.symbols().declaration_symbol(declaration)
    }

    /// Return the DIR type table.
    pub(crate) fn types(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return the DIR decorator table.
    pub(crate) fn decorators(&self) -> &dir::DecoratorTable<'static> {
        &self.decorators
    }

    /// Return the DIR generic table.
    pub(crate) fn generics(&self) -> &dir::GenericTable<'static> {
        &self.generics
    }

    /// Return the DIR definition table.
    pub(crate) fn definitions(&self) -> &dir::DefinitionTable<'static> {
        &self.definitions
    }

    /// Return the DIR resolution table.
    pub(crate) fn resolutions(&self) -> &dir::ResolutionTable<'static> {
        &self.resolutions
    }

    /// Return the DIR module table.
    pub(crate) fn modules(&self) -> &dir::ModuleTable<'static> {
        &self.modules
    }

    /// Return the DIR export table.
    pub(crate) fn exports(&self) -> &dir::ExportTable {
        &self.exported.exports
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(&self) -> dir::LocalScopeId {
        self.namespace_scope
    }

    /// Return the declared or inferred type id for a node.
    pub(crate) fn node_type_id(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);

        self.types().get_node_type_id(global_node_id)
    }
}
