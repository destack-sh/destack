use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactOutcome, ArtifactVersion, DirBound, DirCheckedModule, DirExpanded,
    DirExported, DirImported, DirParsed, GlobalEnvironment,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderResult, Repository, Revision};
use destack_source::{File, FileId, ModuleId, ProfileId, SourceIndex};

use crate::Module;

/// Query context anchored to one module profile.
#[derive(Debug)]
pub struct ModuleQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The source module DIR.
    parsed: Arc<DirParsed>,
    /// The bound module DIR.
    bound: Arc<DirBound>,
    /// The expanded module DIR.
    expanded: Arc<DirExpanded>,
    /// The exported module DIR.
    exported: Arc<DirExported>,
    /// The expanded binding table.
    bindings: dir::BindingTable<'static>,
    /// The expanded module table.
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
    /// The source file token spans.
    tokens: Vec<dir::TokenSpan>,
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

/// Artifact payloads required to build one module query context.
struct ModuleQueryArtifacts {
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
    /// The checked module artifact.
    checked: Arc<DirCheckedModule>,
    /// The profile global environment.
    global_environment: Arc<GlobalEnvironment>,
}

impl<'a> ModuleQueryContext<'a> {
    /// Build one module context from resolved artifacts.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        profile_id: ProfileId,
        module_id: ModuleId,
        file_id: FileId,
        artifacts: ModuleQueryArtifacts,
        tokens: Vec<dir::TokenSpan>,
    ) -> Self {
        // compose expanded table views
        let bindings = artifacts.expanded.binding_table(&artifacts.bound);
        let modules = artifacts.expanded.module_table(&artifacts.imported);

        // compose checked table views
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
            bound: artifacts.bound,
            expanded: artifacts.expanded,
            exported: artifacts.exported,
            bindings,
            modules,
            types,
            decorators,
            generics,
            definitions,
            resolutions,
            tokens,
            global_environment: artifacts.global_environment,
            strings: repository.string_pool().as_ref(),
            revision,
            profile_id,
            module_id,
            file_id,
        }
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

    /// Return the file id for this module context.
    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return one source file, treating missing repository storage as an invariant failure.
    pub(crate) fn read_file(&self, file_id: FileId) -> Arc<File> {
        self.repository()
            .file(self.revision(), file_id)
            .unwrap_or_else(|error| panic!("failed to read source file {file_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing source file {file_id:?}"))
    }

    /// Return this module's source file.
    pub(crate) fn source_file(&self) -> Arc<File> {
        self.read_file(self.file_id())
    }

    /// Return the queried module profile.
    pub fn module(&self) -> Module {
        Module {
            module_id: self.module_id,
            profile_id: self.profile_id,
        }
    }

    /// Return a module context for another module in this profile.
    pub(crate) fn module_context(&self, module_id: ModuleId) -> ModuleQueryContext<'a> {
        ready_module_query_context(self.repository, self.revision, module_id, self.profile_id)
    }

    /// Return the parsed DIR tree.
    pub(crate) fn tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }

    /// Return the parsed DIR source index.
    pub(crate) fn source_index(&self) -> &SourceIndex {
        &self.tree().source_index
    }

    /// Return the parsed DIR parent index.
    pub(crate) fn parents(&self) -> &dir::NodeParentIndex {
        self.tree().parents()
    }

    /// Return the main token stream for this source file.
    pub(crate) fn tokens(&self) -> &[dir::TokenSpan] {
        &self.tokens
    }

    /// Return the source comments.
    pub(crate) fn comments(&self) -> &[dir::Comment] {
        self.parsed
            .file(self.file_id)
            .unwrap_or_else(|| panic!("missing parsed file for {:?}", self.file_id))
            .comments
            .as_slice()
    }

    /// Return the visible DIR tree view.
    pub(crate) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(self.tree(), std::slice::from_ref(&self.expanded.patch))
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
    pub(crate) fn symbols(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the symbol declared by one local node when bound.
    pub(crate) fn node_symbol(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalSymbolId> {
        let declaration = node_id.into_global(self.module_id);

        self.symbols().declaration_symbol(declaration)
    }

    /// Return the lexical scope attached to one local node when bound.
    pub(crate) fn node_scope(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalScope> {
        let node_id = node_id.into_global(self.module_id);

        self.symbols().scope_for_node(node_id)
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

    /// Return the top-level roots for this source file.
    pub(crate) fn roots(&self) -> Option<&[dir::LocalNodeId<dir::Expression>]> {
        self.parsed
            .file(self.file_id)
            .map(|file| file.roots.as_slice())
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(&self) -> dir::LocalScopeId {
        self.bound.namespace_scope
    }

    /// Return the declared or inferred type id for a node.
    pub(crate) fn node_type_id(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);

        self.types().get_node_type_id(global_node_id)
    }

    /// Read one checked global type through its owning module context.
    pub(crate) fn read_global_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &ModuleQueryContext<'_>) -> R,
    ) -> R {
        if type_id.module_id == self.module_id {
            let ty = self.types.get_type(type_id.local_id);

            return read(&ty, self);
        }

        let type_module = self.module_context(type_id.module_id);
        let ty = type_module.types.get_type(type_id.local_id);

        read(&ty, &type_module)
    }

    /// Strip local form wrappers from one checked global type id.
    pub(crate) fn unwrap_global_form_payload_type_id(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> dir::GlobalTypeId {
        let mut current = type_id;

        loop {
            let next = self.read_global_type(current, |ty, _| match ty {
                dir::Type::Form(form) => Some(form.value),
                _ => None,
            });
            let Some(next) = next else {
                return current;
            };

            current = next;
        }
    }

    /// Return the global environment for this query profile.
    pub(crate) fn global_environment(&self) -> &GlobalEnvironment {
        self.global_environment.as_ref()
    }
}

/// Return a module query context for a module profile.
pub fn module_query_context(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<ModuleQueryContext<'_>> {
    // require checked DIR for the requested module profile
    let checked_key = ArtifactKey::dir_checked(module_id, profile_id);
    let checked_version = available_artifact_version(repository, revision, checked_key)?;

    // require the matching global environment
    let global_environment_key = ArtifactKey::global_environment(profile_id);
    let global_environment_version =
        available_artifact_version(repository, revision, global_environment_key)?;

    Some(module_query_context_exact(
        repository,
        revision,
        module_id,
        profile_id,
        checked_version,
        global_environment_version,
    ))
}

/// Return a module query context from exact checked and environment artifacts.
pub fn module_query_context_exact(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
    checked_version: ArtifactVersion,
    global_environment_version: ArtifactVersion,
) -> ModuleQueryContext<'_> {
    // read module metadata and artifact table
    let module = repository
        .module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to read module {module_id:?}: {error}"))
        .unwrap_or_else(|| panic!("missing module {module_id:?}"));
    let artifacts = repository.artifact_table();

    // resolve exact source artifacts
    let parsed_key = ArtifactKey::dir_parsed(module.id);
    let parsed_version = ready_artifact_version(repository, revision, parsed_key);
    let bound_key = ArtifactKey::dir_bound(module.id, profile_id);
    let bound_version = ready_artifact_version(repository, revision, bound_key);
    let imported_key = ArtifactKey::dir_imported(module.id, profile_id);
    let imported_version = ready_artifact_version(repository, revision, imported_key);
    let expanded_key = ArtifactKey::dir_expanded(module.id, profile_id);
    let expanded_version = ready_artifact_version(repository, revision, expanded_key);
    let exported_key = ArtifactKey::dir_exported(module.id, profile_id);
    let exported_version = ready_artifact_version(repository, revision, exported_key);

    // resolve source and profile DIR payloads
    let parsed = artifacts
        .dir_parsed(&parsed_version)
        .unwrap_or_else(|| panic!("missing parsed DIR payload: {parsed_version:?}"));
    let bound = artifacts
        .dir_bound(&bound_version)
        .unwrap_or_else(|| panic!("missing bound DIR payload: {bound_version:?}"));
    let imported = artifacts
        .dir_imported(&imported_version)
        .unwrap_or_else(|| panic!("missing imported DIR payload: {imported_version:?}"));
    let expanded = artifacts
        .dir_expanded(&expanded_version)
        .unwrap_or_else(|| panic!("missing expanded DIR payload: {expanded_version:?}"));
    let exported = artifacts
        .dir_exported(&exported_version)
        .unwrap_or_else(|| panic!("missing exported DIR payload: {exported_version:?}"));
    let checked = artifacts
        .dir_checked(&checked_version)
        .unwrap_or_else(|| panic!("missing checked DIR payload: {checked_version:?}"));

    // resolve the checked component payload for this module
    let checked_component_key =
        ArtifactKey::dir_checked_component(checked.entry, checked.component, profile_id);
    let checked_component_version =
        ready_artifact_version(repository, revision, checked_component_key);
    let checked_component = artifacts
        .dir_checked_component(&checked_component_version)
        .unwrap_or_else(|| {
            panic!("missing checked DIR component payload: {checked_component_version:?}")
        });
    let checked = Arc::new(
        checked_component
            .module(module.id)
            .unwrap_or_else(|| panic!("checked DIR component missing module {:?}", module.id))
            .checked
            .clone(),
    );

    // resolve the profile global environment payload
    let global_environment = artifacts
        .global_environment(&global_environment_version)
        .unwrap_or_else(|| {
            panic!("missing global environment payload: {global_environment_version:?}")
        });

    // materialize source token spans
    let tokens = parsed
        .file(module.file_id)
        .unwrap_or_else(|| panic!("missing parser output for {:?}", module.file_id))
        .iter_token_spans()
        .collect();
    let artifacts = ModuleQueryArtifacts {
        parsed,
        bound,
        imported,
        expanded,
        exported,
        checked,
        global_environment,
    };

    ModuleQueryContext::new(
        repository,
        revision,
        profile_id,
        module.id,
        module.file_id,
        artifacts,
        tokens,
    )
}

/// Return a module context from ready checked artifacts.
pub(super) fn ready_module_query_context(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> ModuleQueryContext<'_> {
    // resolve ready checked DIR and global environment versions
    let checked_key = ArtifactKey::dir_checked(module_id, profile_id);
    let checked_version = ready_artifact_version(repository, revision, checked_key);
    let global_environment_key = ArtifactKey::global_environment(profile_id);
    let global_environment_version =
        ready_artifact_version(repository, revision, global_environment_key);

    module_query_context_exact(
        repository,
        revision,
        module_id,
        profile_id,
        checked_version,
        global_environment_version,
    )
}

/// Return one available ready artifact version.
pub(super) fn available_artifact_version(
    repository: &Repository,
    revision: Revision,
    key: ArtifactKey,
) -> Option<ArtifactVersion> {
    let version = repository
        .artifact_binding(revision, &key)
        .unwrap_or_else(|error| panic!("failed to read artifact binding {key:?}: {error}"))?;
    let outcome = repository.artifact_table().outcome(&version);

    matches!(outcome, Some(ArtifactOutcome::Ok)).then_some(version)
}

/// Return one ready artifact version.
pub(super) fn ready_artifact_version(
    repository: &Repository,
    revision: Revision,
    key: ArtifactKey,
) -> ArtifactVersion {
    available_artifact_version(repository, revision, key)
        .unwrap_or_else(|| panic!("missing ready artifact binding: {key:?}"))
}

/// Build one module query context for a provider attempt.
pub(crate) fn provide_module_query_context<'a>(
    repository: &'a Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
    artifacts: &ArtifactReader<'_>,
) -> ProviderResult<ModuleQueryContext<'a>> {
    // read module metadata for artifact lookup and file ownership
    let module = repository
        .module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to read module {module_id:?}: {error}"))
        .unwrap_or_else(|| panic!("missing module {module_id:?}"));

    // read exact source and profile artifacts
    let parsed = artifacts.dir_parsed(module.id)?;
    let bound = artifacts.dir_bound(module.id, profile_id)?;
    let imported = artifacts.dir_imported(module.id, profile_id)?;
    let expanded = artifacts.dir_expanded(module.id, profile_id)?;
    let exported = artifacts.dir_exported(module.id, profile_id)?;
    let checked = artifacts.dir_checked(module.id, profile_id)?;
    let global_environment = artifacts.global_environment(profile_id)?;

    // materialize source token spans
    let tokens = parsed
        .file(module.file_id)
        .unwrap_or_else(|| panic!("missing parser output for {:?}", module.file_id))
        .iter_token_spans()
        .collect();
    let artifacts = ModuleQueryArtifacts {
        parsed,
        bound,
        imported,
        expanded,
        exported,
        checked,
        global_environment,
    };

    Ok(ModuleQueryContext::new(
        repository,
        revision,
        profile_id,
        module.id,
        module.file_id,
        artifacts,
        tokens,
    ))
}
