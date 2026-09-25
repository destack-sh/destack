use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, OnceLock};

use tspp_artifact::{
    ArtifactKey, DirAnalyzed, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded,
    DirImported, DirMaterialized, DirParsed, DirParsedFile, DirResolved, DirView,
};
use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_repository::{ArtifactReader, ProviderError, Repository, Revision};
use tspp_source::{File, FileId, ModuleId, ProfileId, SourceIndex, Span};

use crate::{Module, QueryError, QueryResult};

/// Query context anchored to one module profile.
pub struct ModuleQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this context.
    revision: Revision,
    /// The profile used for this context.
    profile_id: ProfileId,
    /// The module id.
    module_id: ModuleId,
    /// The function that provides artifacts read by this query.
    require_artifacts: &'a (dyn Fn(&[ArtifactKey]) -> QueryResult<()> + Sync),
    /// Shared repository strings.
    strings: &'a StringPool,
    /// The parsed module artifact.
    parsed: OnceLock<Result<Arc<DirParsed>, ProviderError>>,
    /// The resolved import and source-reference artifact.
    resolved: OnceLock<Result<Arc<DirResolved>, ProviderError>>,
    /// The stages through expansion, read once.
    expanded_stages: OnceLock<Result<DirView, ProviderError>>,
    /// The stages through the analyzed one with their tables stacked once.
    stages: OnceLock<Result<DirView, ProviderError>>,
}

impl Debug for ModuleQueryContext<'_> {
    /// Format the visible module query state.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModuleQueryContext")
            .field("revision", &self.revision)
            .field("profile_id", &self.profile_id)
            .field("module_id", &self.module_id)
            .finish_non_exhaustive()
    }
}

impl<'a> ModuleQueryContext<'a> {
    /// Create one module query context.
    pub fn new(
        repository: &'a Repository,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        require_artifacts: &'a (dyn Fn(&[ArtifactKey]) -> QueryResult<()> + Sync),
    ) -> Self {
        Self {
            repository,
            revision,
            profile_id,
            module_id,
            require_artifacts,
            strings: repository.string_pool().as_ref(),
            parsed: OnceLock::new(),
            resolved: OnceLock::new(),
            expanded_stages: OnceLock::new(),
            stages: OnceLock::new(),
        }
    }

    /// Read one artifact payload.
    fn read_artifact<'b, T>(
        &self,
        key: ArtifactKey,
        artifact: &'b OnceLock<Result<Arc<T>, ProviderError>>,
        read: impl FnOnce(&ArtifactReader<'_>) -> Result<Arc<T>, ProviderError>,
    ) -> QueryResult<&'b T> {
        if let Some(artifact) = artifact.get() {
            return match artifact {
                Ok(artifact) => Ok(artifact.as_ref()),
                Err(error) => Err(QueryError::from(error.clone())),
            };
        }

        // require and read the exact artifact once
        (self.require_artifacts)(&[key])?;
        let artifact = artifact.get_or_init(|| {
            let reader = ArtifactReader::new(self.repository, self.revision);

            read(&reader)
        });

        match artifact {
            Ok(artifact) => Ok(artifact.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return the parsed module artifact.
    fn parsed(&self) -> QueryResult<&DirParsed> {
        self.read_artifact(
            ArtifactKey::dir_parsed(self.module_id),
            &self.parsed,
            |reader| reader.read::<DirParsed>(self.module_id),
        )
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
    pub(crate) fn source_index(&self) -> QueryResult<&SourceIndex> {
        Ok(&self.parsed()?.tree.source_index)
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
            .parsed()?
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

    /// Return the tree through expansion.
    pub(crate) fn view(&self) -> QueryResult<dir::View<'_>> {
        Ok(self.expanded_stages()?.tree())
    }

    /// Return the resolved import and source-reference DIR.
    pub(crate) fn resolved(&self) -> QueryResult<&DirResolved> {
        self.read_artifact(
            ArtifactKey::dir_resolved(self.module_id, self.profile_id),
            &self.resolved,
            |reader| reader.read::<DirResolved>((self.module_id, self.profile_id)),
        )
    }

    /// Return the cumulative DIR binding table.
    pub(crate) fn bindings(&self) -> QueryResult<&dir::BindingTable<'static>> {
        Ok(self.stages()?.bindings())
    }

    /// Return the symbol declared by one local node when bound.
    pub(crate) fn node_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<dir::LocalSymbolId>> {
        let declaration = node_id.into_global(self.module_id);

        Ok(self.bindings()?.declaration_symbol(declaration))
    }

    /// Return the cumulative DIR type table.
    pub(crate) fn types(&self) -> QueryResult<&dir::TypeTable<'static>> {
        Ok(self.stages()?.types())
    }

    /// Return the cumulative DIR static table.
    pub(crate) fn statics(&self) -> QueryResult<&dir::StaticTable<'static>> {
        Ok(self.stages()?.statics())
    }

    /// Return the cumulative DIR decorator table.
    pub(crate) fn decorators(&self) -> QueryResult<&dir::DecoratorTable<'static>> {
        Ok(self.stages()?.decorators())
    }

    /// Return the cumulative DIR generic table.
    pub(crate) fn generics(&self) -> QueryResult<&dir::GenericTable<'static>> {
        Ok(self.stages()?.generics())
    }

    /// Return the cumulative DIR definition table.
    pub(crate) fn definitions(&self) -> QueryResult<&dir::DefinitionTable<'static>> {
        Ok(self.stages()?.definitions())
    }

    /// Return the cumulative DIR decision table.
    pub(crate) fn decisions(&self) -> QueryResult<&dir::DecisionTable<'static>> {
        Ok(self.stages()?.decisions())
    }

    /// Return the cumulative DIR resolution table.
    pub(crate) fn resolutions(&self) -> QueryResult<&dir::ResolutionTable<'static>> {
        Ok(self.stages()?.resolutions())
    }

    /// Return the cumulative DIR member table.
    pub(crate) fn members(&self) -> QueryResult<&dir::MemberTable<'static>> {
        Ok(self.stages()?.members())
    }

    /// Return the cumulative DIR module table.
    pub(crate) fn modules(&self) -> QueryResult<&dir::ModuleTable<'static>> {
        Ok(self.stages()?.modules())
    }

    /// Return the stages through expansion, read once.
    fn expanded_stages(&self) -> QueryResult<&DirView> {
        let key = (self.module_id, self.profile_id);
        (self.require_artifacts)(&[
            ArtifactKey::dir_parsed(self.module_id),
            ArtifactKey::dir_bound(self.module_id, self.profile_id),
            ArtifactKey::dir_imported(self.module_id, self.profile_id),
            ArtifactKey::dir_expanded(self.module_id, self.profile_id),
        ])?;
        let stages = self.expanded_stages.get_or_init(|| {
            let reader = ArtifactReader::new(self.repository, self.revision);

            Ok(DirView::expanded(
                reader.read::<DirParsed>(self.module_id)?,
                reader.read::<DirBound>(key)?,
                reader.read::<DirImported>(key)?,
                reader.read::<DirExpanded>(key)?,
            ))
        });

        match stages {
            Ok(stages) => Ok(stages),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return the stages through the analyzed one with their tables stacked, read once.
    fn stages(&self) -> QueryResult<&DirView> {
        let key = (self.module_id, self.profile_id);
        (self.require_artifacts)(&[
            ArtifactKey::dir_parsed(self.module_id),
            ArtifactKey::dir_bound(self.module_id, self.profile_id),
            ArtifactKey::dir_imported(self.module_id, self.profile_id),
            ArtifactKey::dir_expanded(self.module_id, self.profile_id),
            ArtifactKey::dir_resolved(self.module_id, self.profile_id),
            ArtifactKey::dir_declared(self.module_id, self.profile_id),
            ArtifactKey::dir_elaborated(self.module_id, self.profile_id),
            ArtifactKey::dir_checked(self.module_id, self.profile_id),
            ArtifactKey::dir_materialized(self.module_id, self.profile_id),
            ArtifactKey::dir_analyzed(self.module_id, self.profile_id),
        ])?;
        let view = self.stages.get_or_init(|| {
            let reader = ArtifactReader::new(self.repository, self.revision);

            Ok(DirView::analyzed(
                reader.read::<DirParsed>(self.module_id)?,
                reader.read::<DirBound>(key)?,
                reader.read::<DirImported>(key)?,
                reader.read::<DirExpanded>(key)?,
                reader.read::<DirResolved>(key)?,
                reader.read::<DirDeclared>(key)?,
                reader.read::<DirElaborated>(key)?,
                reader.read::<DirChecked>(key)?,
                reader.read::<DirMaterialized>(key)?,
                reader.read::<DirAnalyzed>(key)?,
            ))
        });

        match view {
            Ok(view) => Ok(view),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(&self) -> QueryResult<dir::LocalScopeId> {
        Ok(self.expanded_stages()?.bound.namespace_scope)
    }

    /// Return the declared or inferred type id for a node.
    pub(crate) fn node_type_id(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<dir::GlobalTypeId>> {
        let global_node_id = node_id.into_global(self.module_id);

        Ok(self.types()?.get_node_type_id(global_node_id))
    }
}
