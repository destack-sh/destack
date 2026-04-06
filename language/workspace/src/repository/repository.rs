use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactCache, ArtifactCacheLayout, ArtifactStore, CacheStore, DiskCacheStore, ProfileKey,
};
use destack_core::StringPool;
use destack_source::{
    DiagnosticCollection, File, FileId, FileStore, FileSystem, FileType, ModuleId, PackageId,
    PhysicalFileSystem, PrintOptions, ProfileId, TargetId, Uri,
};
use parking_lot::RwLock;

use crate::repository::{
    Builtins, FileContentId, FileContentStore, QueryIndex, Ref, RepositoryError, RepositoryOptions,
    Revision, RevisionState, SourceMap, discover_workspace_root,
};
use crate::{
    DestackDeclaration, FormatterOptions, LinterOptions, TsConfigOptions, Workspace, WorkspaceKind,
    resolve_cache_root,
};
/// Tsconfig context for one module profile decision.
#[derive(Debug, Clone)]
pub(crate) struct ModuleTsConfigContext {
    /// The normalized tsconfig options.
    pub(crate) options: TsConfigOptions,
    /// The tsconfig directory for resolving relative paths.
    pub(crate) directory: PathBuf,
}

/// One repository with lineage, sources, artifacts, and shared inputs.
#[derive(Debug)]
pub struct Repository {
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The workspace root directory.
    pub(crate) root: PathBuf,
    /// The working directory for repository-relative operations.
    pub cwd: PathBuf,
    /// Repository defaults for downstream tools.
    pub(crate) options: RepositoryOptions,

    /// Shared string pool.
    pub strings: Arc<StringPool>,
    /// Shared persistent cache backend.
    pub(crate) cache_store: Arc<dyn CacheStore>,
    /// The file system backing repository discovery and loads.
    pub(crate) fs: Arc<dyn FileSystem>,
    /// The logical path for each file id.
    pub(crate) logical_path_by_file_id: DashMap<FileId, Arc<str>>, // FUGU #Suspicious: why Arc<str>?
    /// The package id for each target id.
    pub(crate) package_id_by_target_id: DashMap<TargetId, PackageId>, // FUGU #Suspicious: revision-mutable?
    /// The target name for each target id.
    pub(crate) target_name_by_target_id: DashMap<TargetId, Arc<str>>,

    /// The published source revision graph.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionState>>,
    /// The cached workspace view for each retained revision.
    pub(crate) workspaces: DashMap<Revision, Arc<Workspace>>,
    /// The named movable refs.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// Shared immutable source contents.
    pub(crate) file_contents: FileContentStore,
    /// The retained anonymous revision pins.
    pub(crate) pinned_revisions: DashMap<Revision, usize>,
    /// Shared immutable derived artifacts.
    pub(crate) artifacts: Arc<ArtifactStore>,
    /// Shared builtin selection metadata.
    pub builtins: Arc<Builtins>,
    /// Shared query index storage.
    pub(crate) query_index: RwLock<QueryIndex>,
}

impl Repository {
    /// Open one repository at one directory with default options and disk cache.
    pub fn open_root(root: PathBuf) -> Self {
        let repository = Self::new(
            root.clone(),
            RepositoryOptions::default(),
            Arc::new(DiskCacheStore::new()),
            Arc::new(PhysicalFileSystem::new()),
        );
        let reference = Ref::for_workspace_root(&root);

        repository
            .seed_root_source(&reference)
            .unwrap_or_else(|error| panic!("failed to seed root source: {error}"));
        repository
            .builtins
            .install_in_repository(&repository, &reference)
            .unwrap_or_else(|error| panic!("failed to seed builtin source: {error}"));

        repository
    }

    /// Open one repository at one directory and import that directory from one file system.
    pub fn open_root_from_fs(
        root: PathBuf,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, RepositoryError> {
        let repository = Self::new(
            root.clone(),
            RepositoryOptions::default(),
            Arc::new(DiskCacheStore::new()),
            fs,
        );
        let reference = Ref::for_workspace_root(&root);
        let _ = repository.seed_root_source(&reference)?;
        let _ = repository
            .builtins
            .install_in_repository(&repository, &reference)?;
        let _ = repository.import_from_fs(&reference, &root)?;

        Ok(repository)
    }

    /// Open one repository after discovering workspace metadata from one path.
    pub fn open_detected_from_fs(
        path: PathBuf,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, RepositoryError> {
        let root = discover_workspace_root(&fs, &path)?;

        Self::open_root_from_fs(root, fs)
    }

    /// Create one repository from explicit parts.
    pub fn new(
        root: PathBuf,
        options: RepositoryOptions,
        cache: Arc<dyn CacheStore>,
        fs: Arc<dyn FileSystem>,
    ) -> Self {
        let strings = Arc::new(StringPool::new());
        let file_contents = FileContentStore::new();
        let mut builtins = Builtins::empty();

        let revisions = DashMap::new();
        let workspaces = DashMap::new();
        let refs = DashMap::new();
        let pinned_revisions = DashMap::new();
        let cwd = root.clone();
        let workspace_reference = Ref::for_workspace_root(&root);

        // builtin metadata
        builtins.load_intrinsics();

        let repository = Self {
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            root,
            cwd,
            options,
            fs,
            logical_path_by_file_id: DashMap::new(),
            package_id_by_target_id: DashMap::new(),
            target_name_by_target_id: DashMap::new(),
            strings,
            revisions,
            workspaces,
            refs,
            file_contents,
            pinned_revisions,
            artifacts: Arc::new(ArtifactStore::default()),
            builtins: Arc::new(builtins),
            query_index: RwLock::new(QueryIndex::default()),
            cache_store: cache,
        };

        // initial repository revision
        let initial_revision = Arc::new(RevisionState::new(
            smallvec::SmallVec::new(),
            Arc::new(SourceMap::new()),
        ));
        let initial_revision_id = initial_revision.revision();
        repository
            .revisions
            .insert(initial_revision_id, initial_revision);
        repository
            .refs
            .insert(workspace_reference, initial_revision_id);

        repository
    }

    /// Override the default formatter options.
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.formatter = formatter;
        self
    }

    /// Override the default linter options.
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.linter = linter;
        self
    }

    /// Override the backing cache store.
    pub fn with_cache_store(mut self, cache_store: Arc<dyn CacheStore>) -> Self {
        self.cache_store = cache_store;
        self
    }

    /// Override the retained file history depth per pinned revision.
    pub fn with_file_history_limit(mut self, file_history_limit: usize) -> Self {
        self.options.file_history_limit = file_history_limit;
        self
    }

    /// Override the cache directory root.
    pub fn with_cache_directory(mut self, cache_directory: PathBuf) -> Self {
        self.options.cache_directory_override = Some(cache_directory);
        self
    }

    /// Return the repository file system.
    pub fn file_system(&self) -> &Arc<dyn FileSystem> {
        &self.fs
    }

    /// Return the repository string pool.
    pub fn string_pool(&self) -> &Arc<StringPool> {
        &self.strings
    }

    /// Return the repository artifact store.
    pub fn artifact_store(&self) -> &Arc<ArtifactStore> {
        &self.artifacts
    }

    /// Return the repository builtin inputs.
    pub fn builtins(&self) -> &Arc<Builtins> {
        &self.builtins
    }

    /// Load one builtin library module set for one profile.
    pub fn load_builtin_library(
        &self,
        name: &str,
        profile_key: &ProfileKey,
    ) -> Option<Vec<ModuleId>> {
        self.builtins.load_library(name, profile_key)
    }

    /// Return the repository cache store.
    pub fn cache_store(&self) -> &Arc<dyn CacheStore> {
        &self.cache_store
    }

    /// Return the repository formatter options.
    pub fn formatter_options(&self) -> &FormatterOptions {
        &self.formatter
    }

    /// Print one diagnostic collection with revision scoped source context.
    pub fn print_diagnostics(&self, revision: Revision, diagnostics: &DiagnosticCollection) {
        let options = PrintOptions::new()
            .with_line_width(self.formatter.line_width as u32)
            .with_module_count(
                self.workspace_module_ids(revision)
                    .unwrap_or_default()
                    .len(),
            );

        let files = FileStore::new();
        let mut file_ids = diagnostics
            .iter()
            .into_iter()
            .map(|diagnostic| diagnostic.file_id)
            .collect::<Vec<_>>();

        for diagnostic in diagnostics.iter() {
            if let Some(secondary_spans) = diagnostic.secondary_spans.as_ref() {
                file_ids.extend(secondary_spans.iter().map(|span| span.span.file));
            }
        }

        file_ids.sort_unstable();
        file_ids.dedup();

        for file_id in file_ids {
            let Ok(Some(file)) = self.file(revision, file_id) else {
                continue;
            };

            files.insert(file.as_ref().clone());
        }

        destack_source::print_diagnostics(&files, diagnostics, options);
    }

    /// Return the repository linter options.
    pub fn linter_options(&self) -> &LinterOptions {
        &self.linter
    }

    /// Return the repository workspace root.
    pub fn workspace_root(&self) -> &Path {
        &self.root
    }

    /// Return one cached discovered workspace view for one revision.
    pub fn workspace(&self, revision: Revision) -> Result<Arc<Workspace>, RepositoryError> {
        if let Some(workspace) = self.workspaces.get(&revision) {
            return Ok(Arc::clone(workspace.value()));
        }

        // TODO #Cleanup: unify Workspace with the public Package and Module revision snapshots
        // so this cache stops carrying the thinner discovered-only layer
        let workspace_declaration = self.workspace_destack_declaration(revision)?;
        let revision_state = self.revision(revision)?;
        let packages = self.derive_packages(revision_state.source.as_ref());
        let modules = self.derive_modules(revision_state.source.as_ref(), &packages);
        let kind = if packages.len() > 1 {
            WorkspaceKind::Monorepo
        } else {
            WorkspaceKind::SinglePackage
        };

        let workspace = Arc::new(Workspace {
            destack_file_id: workspace_declaration
                .as_ref()
                .map(|declaration| declaration.file_id),
            root: self.root.clone(),
            kind,
            packages,
            modules,
        });
        let entry = self
            .workspaces
            .entry(revision)
            .or_insert_with(|| Arc::clone(&workspace));

        Ok(Arc::clone(entry.value()))
    }

    /// Return the repository default options.
    pub fn options(&self) -> &RepositoryOptions {
        &self.options
    }

    /// Resolve the repository cache directory.
    pub fn cache_directory(&self) -> PathBuf {
        self.resolve_cache_root()
    }

    /// Build one persisted artifact cache layout.
    pub fn artifact_cache_layout(&self, cache_abi: &str) -> ArtifactCacheLayout {
        let cache_root = self.resolve_cache_root();
        let is_shared_root = !cache_root.starts_with(self.workspace_root());

        ArtifactCacheLayout::new(
            &cache_root,
            self.workspace_root(),
            cache_abi,
            is_shared_root,
        )
    }

    /// Build one persisted artifact cache.
    pub fn artifact_cache(&self, cache_abi: &str) -> ArtifactCache<'_> {
        let layout = self.artifact_cache_layout(cache_abi);

        ArtifactCache::new(self.cache_store().as_ref(), &layout)
    }

    /// Resolve one cache root from repository runtime options.
    fn resolve_cache_root(&self) -> PathBuf {
        let cache_directory_override = self.options.cache_directory_override.as_deref();

        resolve_cache_root(self.workspace_root(), cache_directory_override)
    }

    /// Load one `destack.json` declaration from disk when present.
    pub fn load_destack_declaration_for_path(&self, path: &Path) -> Option<DestackDeclaration> {
        let content = self.fs.read_to_string(path).ok()?;
        let file = File::from_text_as_jsonc(
            FileId::from_logical_path(path),
            path.file_name()?.to_string_lossy().to_string(),
            Uri::from_path(path),
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        )
        .ok()?;
        let file = Arc::new(file);

        DestackDeclaration::parse(&file).ok()
    }

    /// Return the stable profile id for one semantic profile key.
    pub fn profile_id(&self, key: ProfileKey) -> ProfileId {
        ProfileId::new(key.stable_hash())
    }

    /// Read the repository query index.
    pub fn with_query_index<R>(&self, read: impl FnOnce(&QueryIndex) -> R) -> R {
        let query_index = self.query_index.read();
        read(&query_index)
    }

    /// Mutate the repository query index.
    pub fn with_query_index_mut<R>(&self, write: impl FnOnce(&mut QueryIndex) -> R) -> R {
        let mut query_index = self.query_index.write();
        write(&mut query_index)
    }

    /// Return the current revision for one ref.
    pub fn current(&self, reference: &Ref) -> Result<Revision, RepositoryError> {
        self.refs
            .get(reference)
            .map(|entry| *entry.value())
            .ok_or_else(|| RepositoryError::MissingRef {
                reference: reference.clone(),
            })
    }

    /// Return one published revision.
    pub fn revision(&self, revision: Revision) -> Result<Arc<RevisionState>, RepositoryError> {
        self.revisions
            .get(&revision)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(RepositoryError::MissingRevision { revision })
    }

    /// Return one shared file content payload by exact content id.
    pub fn file_content_by_id(
        &self,
        content: FileContentId,
    ) -> Result<Arc<destack_source::FileContent>, RepositoryError> {
        self.file_contents
            .get(content)
            .ok_or(RepositoryError::MissingContent { content })
    }
}
