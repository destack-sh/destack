use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactVersion, DirBound, DirExpanded, DirExported, DirParsed, GlobalEnvironment,
    ModuleQueryIndex, WorkspaceQueryIndex,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::{FileId, ModuleId, ProfileId, SourceIndex};

use super::QueryModule;

/// Query context anchored to one module profile.
#[derive(Debug)]
pub struct ModuleQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The source module DIR.
    dir_parsed: Arc<DirParsed>,
    /// The bound module DIR.
    dir_bound: Arc<DirBound>,
    /// The expanded module DIR.
    dir_expanded: Arc<DirExpanded>,
    /// The exported module DIR.
    dir_exported: Arc<DirExported>,
    /// The expanded binding table.
    dir_bindings: dir::BindingTable<'static>,
    /// The expanded module table.
    dir_modules: dir::ModuleTable<'static>,
    /// The checked type table.
    dir_types: dir::TypeTable<'static>,
    /// The checked static table.
    dir_statics: dir::StaticTable<'static>,
    /// The checked generic table.
    dir_generics: dir::GenericTable<'static>,
    /// The checked definition table.
    dir_definitions: dir::DefinitionTable<'static>,
    /// The checked resolution table.
    dir_resolutions: dir::ResolutionTable<'static>,
    /// The source file token spans.
    tokens: Vec<dir::TokenSpan>,
    /// The source file side token spans.
    side_tokens: Vec<dir::TokenSpan>,
    /// The profile global environment.
    global_environment: Arc<GlobalEnvironment>,
    /// Shared repository strings.
    strings: &'a StringPool,
    /// The revision used for this context.
    revision: Revision,
    /// The profile used for this context.
    profile_id: ProfileId,
    /// The module id.
    module_id: ModuleId,
    /// The source file id.
    file_id: FileId,
}

/// Query context anchored to a workspace revision.
#[derive(Debug)]
pub struct WorkspaceQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this query.
    revision: Revision,
    /// The query indexes included in this workspace query.
    indexes: Vec<WorkspaceQueryProfile>,
}

/// Query index payloads for one profile.
#[derive(Debug)]
pub(crate) struct WorkspaceQueryProfile {
    /// The profile covered by these indexes.
    profile_id: ProfileId,
    /// The module query indexes referenced by the workspace index.
    modules: Vec<Arc<ModuleQueryIndex>>,
}

impl WorkspaceQueryProfile {
    /// Return the profile covered by these indexes.
    pub(crate) fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the module query indexes for this profile.
    pub(crate) fn modules(&self) -> &[Arc<ModuleQueryIndex>] {
        &self.modules
    }
}

/// DIR-facing query surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DirQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The module id for this dir view.
    module_id: ModuleId,
    /// The revision for this dir view.
    revision: Revision,
    /// The profile for this dir view.
    profile_id: ProfileId,
    /// The source file id for this dir view.
    file_id: FileId,
    /// The parsed DIR tree.
    tree: &'a dir::Tree,
    /// The parsed DIR parent index.
    parents: &'a dir::NodeParentIndex,
    /// Top-level expressions for this source file.
    roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The expanded DIR patch.
    patch: &'a dir::Patch,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
    /// The main token stream for this source file.
    tokens: &'a [dir::TokenSpan],
    /// The side token stream for this source file.
    side_tokens: &'a [dir::TokenSpan],
    /// The expanded binding table.
    symbols: &'a dir::BindingTable<'static>,
    /// The expanded module table.
    modules: &'a dir::ModuleTable<'static>,
    /// The exported symbol table.
    exports: &'a dir::ExportTable,
    /// The checked type table.
    types: &'a dir::TypeTable<'static>,
    /// The checked generic table.
    generics: &'a dir::GenericTable<'static>,
    /// The checked definition table.
    definitions: &'a dir::DefinitionTable<'static>,
    /// The checked resolution table.
    resolutions: &'a dir::ResolutionTable<'static>,
    /// Shared repository strings.
    strings: &'a StringPool,
}

impl<'a> DirQueryContext<'a> {
    /// Return the module id for this dir view.
    pub(crate) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the revision for this dir view.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the repository for this dir view.
    pub(crate) fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return a module context for another module in the same revision and profile.
    pub(crate) fn module_context(&self, module_id: ModuleId) -> Option<ModuleQueryContext<'a>> {
        read_module_query_context(self.repository, self.revision, module_id, self.profile_id)
    }

    /// Return the source file id for this dir view.
    pub(crate) fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the parsed DIR tree.
    pub(crate) fn tree(self) -> &'a dir::Tree {
        self.tree
    }

    /// Return the parsed DIR source index.
    pub(crate) fn source_index(self) -> &'a SourceIndex {
        &self.tree.source_index
    }

    /// Return the parsed DIR parent index.
    pub(crate) fn parents(self) -> &'a dir::NodeParentIndex {
        self.parents
    }

    /// Return the main token stream for this source file.
    pub(crate) fn tokens(self) -> &'a [dir::TokenSpan] {
        self.tokens
    }

    /// Return the side token stream for this source file.
    pub(crate) fn side_tokens(self) -> &'a [dir::TokenSpan] {
        self.side_tokens
    }

    /// Return the visible DIR tree view.
    pub(crate) fn view(self) -> dir::View<'a> {
        dir::View::with_patches(self.tree, std::slice::from_ref(self.patch))
    }

    /// Return whether one DIR symbol is visible in this query view.
    pub(crate) fn symbol_is_visible(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        if symbol_id.module_id != self.module_id {
            return true;
        }

        let symbol = self.symbols().get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return true;
        };

        self.view().is_visible(declaration.local_id)
    }

    /// Return the DIR symbol table.
    pub(crate) fn symbols(self) -> &'a dir::BindingTable<'static> {
        self.symbols
    }

    /// Return the symbol declared by one local node when bound.
    pub(crate) fn symbol_for_node(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalSymbolId> {
        let declaration = node_id.into_global(self.module_id);

        self.symbols().symbol_for_declaration(declaration)
    }

    /// Return the lexical scope attached to one local node when bound.
    pub(crate) fn scope_for_node(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalScope> {
        let node_id = node_id.into_global(self.module_id);

        self.symbols().scope_for_node(node_id)
    }

    /// Return the DIR type table.
    pub(crate) fn types(self) -> &'a dir::TypeTable<'static> {
        self.types
    }

    /// Return the DIR generic table.
    pub(crate) fn generics(self) -> &'a dir::GenericTable<'static> {
        self.generics
    }

    /// Return the DIR definition table.
    pub(crate) fn definitions(self) -> &'a dir::DefinitionTable<'static> {
        self.definitions
    }

    /// Return the DIR resolution table.
    pub(crate) fn resolutions(self) -> &'a dir::ResolutionTable<'static> {
        self.resolutions
    }

    /// Return the DIR module table.
    pub(crate) fn modules(self) -> &'a dir::ModuleTable<'static> {
        self.modules
    }

    /// Return the DIR export table.
    pub(crate) fn exports(self) -> &'a dir::ExportTable {
        self.exports
    }

    /// Return the top-level roots for this source file.
    pub(crate) fn roots(self) -> &'a [dir::LocalNodeId<dir::Expression>] {
        self.roots
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(self) -> &'a StringPool {
        self.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(&self) -> dir::LocalScopeId {
        self.namespace_scope
    }

    /// Get the inferred type id for a node.
    pub(crate) fn expression_type_id(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types().get_node_type_id(global_node_id)
    }

    /// Get the declared or inferred type id for a node.
    pub(crate) fn node_type_id(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types().get_node_type_id(global_node_id)
    }
}

impl<'a> ModuleQueryContext<'a> {
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

    /// Return the file id for this module context.
    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the queried module profile.
    pub fn query_module(&self) -> QueryModule {
        QueryModule {
            module_id: self.module_id,
            profile_id: self.profile_id,
        }
    }

    /// Return a module context for another module in the same revision and profile.
    pub(crate) fn module_context(&self, module_id: ModuleId) -> Option<ModuleQueryContext<'a>> {
        read_module_query_context(self.repository, self.revision, module_id, self.profile_id)
    }

    /// Read one checked global type through its owning module context.
    pub(crate) fn with_global_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &ModuleQueryContext<'_>) -> R,
    ) -> Option<R> {
        if type_id.module_id == self.module_id {
            let ty = self.dir_types.get_type_maybe(type_id.local_id)?;

            return Some(read(ty, self));
        }

        let context = self.module_context(type_id.module_id)?;
        let ty = context.dir_types.get_type_maybe(type_id.local_id)?;

        Some(read(ty, &context))
    }

    /// Read one checked global static value through its owning module context.
    pub(crate) fn with_global_static<R>(
        &self,
        static_id: dir::GlobalStaticId,
        read: impl FnOnce(&dir::StaticTerm, &ModuleQueryContext<'_>) -> R,
    ) -> Option<R> {
        if static_id.module_id == self.module_id {
            let value = self.dir_statics.get_static_maybe(static_id.local_id)?;

            return Some(read(value, self));
        }

        let context = self.module_context(static_id.module_id)?;
        let value = context.dir_statics.get_static_maybe(static_id.local_id)?;

        Some(read(value, &context))
    }

    /// Strip local form wrappers from one checked global type id.
    pub(crate) fn unwrap_global_form_payload_type_id(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Option<dir::GlobalTypeId> {
        let mut current = type_id;

        loop {
            let next = self.with_global_type(current, |ty, _| match ty {
                dir::Type::Form(form) => Some(form.value),
                _ => None,
            })?;
            let Some(next) = next else {
                return Some(current);
            };

            current = next;
        }
    }

    /// Return the dir query surface.
    pub(crate) fn dir(&self) -> DirQueryContext<'_> {
        let roots = self
            .dir_parsed
            .roots_for_file(self.file_id)
            .unwrap_or_else(|| panic!("query source file was not parsed: {:?}", self.file_id));

        DirQueryContext {
            repository: self.repository,
            module_id: self.module_id,
            revision: self.revision,
            profile_id: self.profile_id,
            file_id: self.file_id,
            tree: &self.dir_parsed.tree,
            parents: &self.dir_parsed.parents,
            roots,
            patch: &self.dir_expanded.patch,
            namespace_scope: self.dir_bound.namespace_scope,
            tokens: &self.tokens,
            side_tokens: &self.side_tokens,
            symbols: &self.dir_bindings,
            modules: &self.dir_modules,
            exports: &self.dir_exported.exports,
            types: &self.dir_types,
            generics: &self.dir_generics,
            definitions: &self.dir_definitions,
            resolutions: &self.dir_resolutions,
            strings: self.strings,
        }
    }

    /// Return the global environment for this query profile.
    pub(crate) fn global_environment(&self) -> &GlobalEnvironment {
        self.global_environment.as_ref()
    }
}

impl<'a> WorkspaceQueryContext<'a> {
    /// Return the repository for this workspace context.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the revision for this workspace context.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the profiles included in this workspace context.
    pub fn profile_ids(&self) -> impl Iterator<Item = ProfileId> + '_ {
        self.indexes.iter().map(WorkspaceQueryProfile::profile_id)
    }

    /// Return the workspace query index payloads.
    pub(crate) fn indexes(&self) -> &[WorkspaceQueryProfile] {
        &self.indexes
    }

    /// Return a module context in this workspace revision.
    pub(crate) fn module_context(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<ModuleQueryContext<'a>> {
        read_module_query_context(self.repository, self.revision, module_id, profile_id)
    }
}

/// Get a workspace query context for explicit profiles.
pub fn workspace_query_context<'a>(
    repository: &'a Repository,
    revision: Revision,
    indexes: impl IntoIterator<Item = (ProfileId, Arc<WorkspaceQueryIndex>)>,
) -> Option<WorkspaceQueryContext<'a>> {
    let mut query_indexes = Vec::new();

    for (profile_id, index) in indexes {
        let modules = module_query_indexes(repository, &index)?;

        query_indexes.push(WorkspaceQueryProfile {
            profile_id,
            modules,
        });
    }

    Some(WorkspaceQueryContext {
        repository,
        revision,
        indexes: query_indexes,
    })
}

/// Get a module query context for a module profile.
pub fn module_query_context(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
) -> Option<ModuleQueryContext<'_>> {
    read_module_query_context(repository, revision, module_id, profile)
}

/// Get a module query context from one exact checked artifact.
pub fn module_query_context_from_checked(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
    checked_version: ArtifactVersion,
    global_environment_version: ArtifactVersion,
) -> Option<ModuleQueryContext<'_>> {
    read_module_query_context_from_checked(
        repository,
        revision,
        module_id,
        profile,
        checked_version,
        global_environment_version,
    )
}

/// Read a module query context from cached artifacts.
fn read_module_query_context(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
) -> Option<ModuleQueryContext<'_>> {
    let checked_key = ArtifactKey::dir_checked(module_id, profile);
    let checked_version = repository
        .artifact_version(revision, &checked_key)
        .ok()
        .flatten()?;
    let global_environment_key = ArtifactKey::global_environment(profile);
    let global_environment_version = repository
        .artifact_version(revision, &global_environment_key)
        .ok()
        .flatten()?;

    read_module_query_context_from_checked(
        repository,
        revision,
        module_id,
        profile,
        checked_version,
        global_environment_version,
    )
}

/// Read a module query context from cached artifacts.
fn read_module_query_context_from_checked(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
    checked_version: ArtifactVersion,
    global_environment_version: ArtifactVersion,
) -> Option<ModuleQueryContext<'_>> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let artifacts = repository.artifact_store();

    // resolve exact source artifacts
    let parsed_key = ArtifactKey::dir_parsed(module.id);
    let parsed_version = repository
        .artifact_version(revision, &parsed_key)
        .ok()
        .flatten()?;
    let bound_key = ArtifactKey::dir_bound(module.id, profile);
    let bound_version = repository
        .artifact_version(revision, &bound_key)
        .ok()
        .flatten()?;
    let imported_key = ArtifactKey::dir_imported(module.id, profile);
    let imported_version = repository
        .artifact_version(revision, &imported_key)
        .ok()
        .flatten()?;
    let expanded_key = ArtifactKey::dir_expanded(module.id, profile);
    let expanded_version = repository
        .artifact_version(revision, &expanded_key)
        .ok()
        .flatten()?;
    let exported_key = ArtifactKey::dir_exported(module.id, profile);
    let exported_version = repository
        .artifact_version(revision, &exported_key)
        .ok()
        .flatten()?;
    // resolve source and profile dir artifacts
    let dir_parsed = artifacts.dir_parsed(&parsed_version)?;
    let dir_bound = artifacts.dir_bound(&bound_version)?;
    let dir_imported = artifacts.dir_imported(&imported_version)?;
    let dir_expanded = artifacts.dir_expanded(&expanded_version)?;
    let dir_exported = artifacts.dir_exported(&exported_version)?;
    let checked = artifacts.dir_checked(&checked_version)?;
    let checked_component_key =
        ArtifactKey::dir_checked_component(checked.entry, checked.component, profile);
    let checked_component_version = repository
        .artifact_version(revision, &checked_component_key)
        .ok()
        .flatten()?;
    let checked_component = artifacts.dir_checked_component(&checked_component_version)?;
    let dir_checked = checked_component.module(module.id)?.checked.clone();
    let global_environment = artifacts.global_environment(&global_environment_version)?;
    // compose cumulative table views
    let dir_bindings = dir_expanded.binding_table(&dir_bound);
    let dir_modules = dir_expanded.module_table(&dir_imported);
    let dir_types = dir_checked.type_table(&dir_bound, &dir_expanded);
    let dir_statics = dir_checked.static_table(&dir_bound, &dir_expanded);
    let dir_generics = dir_checked.generic_table();
    let dir_definitions = dir_checked.definition_table();
    let dir_resolutions = dir_checked.resolution_table();
    let tokens = dir_parsed
        .iter_token_spans_for_file(module.file_id)?
        .collect();
    let side_tokens = dir_parsed
        .iter_side_token_spans_for_file(module.file_id)?
        .collect();

    Some(ModuleQueryContext {
        repository,
        dir_parsed,
        dir_bound,
        dir_expanded,
        dir_exported,
        dir_bindings,
        dir_modules,
        dir_types,
        dir_statics,
        dir_generics,
        dir_definitions,
        dir_resolutions,
        tokens,
        side_tokens,
        global_environment,
        strings: repository.string_pool().as_ref(),
        revision,
        profile_id: profile,
        module_id: module.id,
        file_id: module.file_id,
    })
}

/// Return all module query indexes referenced by one workspace index.
fn module_query_indexes(
    repository: &Repository,
    workspace: &WorkspaceQueryIndex,
) -> Option<Vec<Arc<ModuleQueryIndex>>> {
    let mut indexes = Vec::with_capacity(workspace.modules.len());

    for version in &workspace.modules {
        let index = repository.artifact_store().module_query_index(version)?;

        indexes.push(index);
    }

    Some(indexes)
}

/// Require and build one query context for a provider attempt.
pub(crate) fn require_module_query_context<'a>(
    repository: &'a Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
    artifacts: &ArtifactReader<'_>,
) -> Result<ModuleQueryContext<'a>, Box<ProviderError>> {
    let module = repository
        .module(revision, module_id)
        .map_err(|error| ProviderError::internal(error.to_string()))?
        .ok_or_else(|| ProviderError::internal(format!("missing module {module_id:?}")))?;

    // require exact source and profile artifacts
    let dir_parsed = artifacts.dir_parsed(module.id)?;
    let dir_bound = artifacts.dir_bound(module.id, profile)?;
    let dir_imported = artifacts.dir_imported(module.id, profile)?;
    let dir_expanded = artifacts.dir_expanded(module.id, profile)?;
    let dir_exported = artifacts.dir_exported(module.id, profile)?;
    let dir_checked = artifacts.dir_checked(module.id, profile)?;
    let global_environment = artifacts.global_environment(profile)?;

    // compose cumulative table views
    let dir_bindings = dir_expanded.binding_table(&dir_bound);
    let dir_modules = dir_expanded.module_table(&dir_imported);
    let dir_types = dir_checked.type_table(&dir_bound, &dir_expanded);
    let dir_statics = dir_checked.static_table(&dir_bound, &dir_expanded);
    let dir_generics = dir_checked.generic_table();
    let dir_definitions = dir_checked.definition_table();
    let dir_resolutions = dir_checked.resolution_table();
    let tokens = dir_parsed
        .iter_token_spans_for_file(module.file_id)
        .ok_or_else(|| ProviderError::internal(format!("missing token stream for {module_id:?}")))?
        .collect();
    let side_tokens = dir_parsed
        .iter_side_token_spans_for_file(module.file_id)
        .ok_or_else(|| {
            ProviderError::internal(format!("missing side token stream for {module_id:?}"))
        })?
        .collect();

    Ok(ModuleQueryContext {
        repository,
        dir_parsed,
        dir_bound,
        dir_expanded,
        dir_exported,
        dir_bindings,
        dir_modules,
        dir_types,
        dir_statics,
        dir_generics,
        dir_definitions,
        dir_resolutions,
        tokens,
        side_tokens,
        global_environment,
        strings: repository.string_pool().as_ref(),
        revision,
        profile_id: profile,
        module_id: module.id,
        file_id: module.file_id,
    })
}
