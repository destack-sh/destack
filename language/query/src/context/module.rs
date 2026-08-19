use std::fmt::{self, Debug, Formatter};
use std::slice;
use std::sync::{Arc, OnceLock};

use destack_artifact::{
    ArtifactKey, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported,
    DirParsed, DirParsedFile, DirResolved,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::{File, FileId, ModuleId, ProfileId, SourceIndex, Span};

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
    /// The bound module artifact.
    bound: OnceLock<Result<Arc<DirBound>, ProviderError>>,
    /// The imported module artifact.
    imported: OnceLock<Result<Arc<DirImported>, ProviderError>>,
    /// The expanded module artifact.
    expanded: OnceLock<Result<Arc<DirExpanded>, ProviderError>>,
    /// The resolved import and source-reference artifact.
    resolved: OnceLock<Result<Arc<DirResolved>, ProviderError>>,
    /// The declared module artifact.
    declared: OnceLock<Result<Arc<DirDeclared>, ProviderError>>,
    /// The elaborated module artifact.
    elaborated: OnceLock<Result<Arc<DirElaborated>, ProviderError>>,
    /// The checked module artifact.
    checked: OnceLock<Result<Arc<DirChecked>, ProviderError>>,
    /// The cumulative binding table.
    bindings: OnceLock<dir::BindingTable<'static>>,
    /// The cumulative module table.
    modules: OnceLock<dir::ModuleTable<'static>>,
    /// The cumulative type table.
    types: OnceLock<dir::TypeTable<'static>>,
    /// The cumulative decorator table.
    decorators: OnceLock<dir::DecoratorTable<'static>>,
    /// The cumulative generic table.
    generics: OnceLock<dir::GenericTable<'static>>,
    /// The cumulative definition table.
    definitions: OnceLock<dir::DefinitionTable<'static>>,
    /// The cumulative resolution table.
    resolutions: OnceLock<dir::ResolutionTable<'static>>,
    decisions: OnceLock<dir::DecisionTable<'static>>,
    /// The checked member table.
    members: OnceLock<dir::MemberTable<'static>>,
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
            bound: OnceLock::new(),
            imported: OnceLock::new(),
            expanded: OnceLock::new(),
            resolved: OnceLock::new(),
            declared: OnceLock::new(),
            elaborated: OnceLock::new(),
            checked: OnceLock::new(),
            bindings: OnceLock::new(),
            modules: OnceLock::new(),
            types: OnceLock::new(),
            decorators: OnceLock::new(),
            generics: OnceLock::new(),
            definitions: OnceLock::new(),
            resolutions: OnceLock::new(),
            decisions: OnceLock::new(),
            members: OnceLock::new(),
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

    /// Return the bound module artifact.
    fn bound(&self) -> QueryResult<&DirBound> {
        self.read_artifact(
            ArtifactKey::dir_bound(self.module_id, self.profile_id),
            &self.bound,
            |reader| reader.read::<DirBound>((self.module_id, self.profile_id)),
        )
    }

    /// Return the imported module artifact.
    fn imported(&self) -> QueryResult<&DirImported> {
        self.read_artifact(
            ArtifactKey::dir_imported(self.module_id, self.profile_id),
            &self.imported,
            |reader| reader.read::<DirImported>((self.module_id, self.profile_id)),
        )
    }

    /// Return the expanded module artifact.
    fn expanded(&self) -> QueryResult<&DirExpanded> {
        self.read_artifact(
            ArtifactKey::dir_expanded(self.module_id, self.profile_id),
            &self.expanded,
            |reader| reader.read::<DirExpanded>((self.module_id, self.profile_id)),
        )
    }

    /// Return the declared module artifact.
    fn declared(&self) -> QueryResult<&DirDeclared> {
        self.read_artifact(
            ArtifactKey::dir_declared(self.module_id, self.profile_id),
            &self.declared,
            |reader| reader.read::<DirDeclared>((self.module_id, self.profile_id)),
        )
    }

    /// Return the elaborated module artifact.
    fn elaborated(&self) -> QueryResult<&DirElaborated> {
        self.read_artifact(
            ArtifactKey::dir_elaborated(self.module_id, self.profile_id),
            &self.elaborated,
            |reader| reader.read::<DirElaborated>((self.module_id, self.profile_id)),
        )
    }

    /// Return the checked module artifact.
    fn checked(&self) -> QueryResult<&DirChecked> {
        self.read_artifact(
            ArtifactKey::dir_checked(self.module_id, self.profile_id),
            &self.checked,
            |reader| reader.read::<DirChecked>((self.module_id, self.profile_id)),
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

    /// Return the visible DIR tree view.
    pub(crate) fn view(&self) -> QueryResult<dir::View<'_>> {
        let expanded = self.expanded()?;
        let parsed = self.parsed()?;

        Ok(dir::View::with_patches(
            &parsed.tree,
            slice::from_ref(&expanded.patch),
        ))
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
        if let Some(bindings) = self.bindings.get() {
            return Ok(bindings);
        }

        let checked = self.checked()?;
        let bound = self.bound()?;
        let expanded = self.expanded()?;
        let declared = self.declared()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .bindings
            .get_or_init(|| checked.binding_table(bound, expanded, declared, elaborated)))
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
        if let Some(types) = self.types.get() {
            return Ok(types);
        }

        let checked = self.checked()?;
        let bound = self.bound()?;
        let expanded = self.expanded()?;
        let declared = self.declared()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .types
            .get_or_init(|| checked.type_table(bound, expanded, declared, elaborated)))
    }

    /// Return the cumulative DIR decorator table.
    pub(crate) fn decorators(&self) -> QueryResult<&dir::DecoratorTable<'static>> {
        if let Some(decorators) = self.decorators.get() {
            return Ok(decorators);
        }

        let checked = self.checked()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .decorators
            .get_or_init(|| checked.decorator_table(elaborated)))
    }

    /// Return the cumulative DIR generic table.
    pub(crate) fn generics(&self) -> QueryResult<&dir::GenericTable<'static>> {
        if let Some(generics) = self.generics.get() {
            return Ok(generics);
        }

        let checked = self.checked()?;
        let declared = self.declared()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .generics
            .get_or_init(|| checked.generic_table(declared, elaborated)))
    }

    /// Return the cumulative DIR definition table.
    pub(crate) fn definitions(&self) -> QueryResult<&dir::DefinitionTable<'static>> {
        if let Some(definitions) = self.definitions.get() {
            return Ok(definitions);
        }

        let elaborated = self.elaborated()?;
        let checked = self.checked()?;

        Ok(self
            .definitions
            .get_or_init(|| checked.definition_table(elaborated)))
    }

    /// Return the cumulative DIR decision table.
    pub(crate) fn decisions(&self) -> QueryResult<&dir::DecisionTable<'static>> {
        if let Some(decisions) = self.decisions.get() {
            return Ok(decisions);
        }

        let checked = self.checked()?;
        let declared = self.declared()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .decisions
            .get_or_init(|| checked.decision_table(declared, elaborated)))
    }

    /// Return the cumulative DIR resolution table.
    pub(crate) fn resolutions(&self) -> QueryResult<&dir::ResolutionTable<'static>> {
        if let Some(resolutions) = self.resolutions.get() {
            return Ok(resolutions);
        }

        let checked = self.checked()?;
        let declared = self.declared()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .resolutions
            .get_or_init(|| checked.resolution_table(declared, elaborated)))
    }

    /// Return the checked DIR member table.
    pub(crate) fn members(&self) -> QueryResult<&dir::MemberTable<'static>> {
        if let Some(members) = self.members.get() {
            return Ok(members);
        }

        let checked = self.checked()?;
        let elaborated = self.elaborated()?;

        Ok(self
            .members
            .get_or_init(|| checked.member_table(elaborated)))
    }

    /// Return the cumulative DIR module table.
    pub(crate) fn modules(&self) -> QueryResult<&dir::ModuleTable<'static>> {
        if let Some(modules) = self.modules.get() {
            return Ok(modules);
        }

        let imported = self.imported()?;
        let expanded = self.expanded()?;

        Ok(self.modules.get_or_init(|| expanded.module_table(imported)))
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(&self) -> QueryResult<dir::LocalScopeId> {
        Ok(self.bound()?.namespace_scope)
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
