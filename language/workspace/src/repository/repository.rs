use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactCache, ArtifactCacheLayout, ArtifactStore, CacheStore, DiskCacheStore, ProfileKey,
};
use destack_core::StringPool;
use destack_source::{
    DiagnosticCollection, FileContentEntry, FileSystem, ModuleId, PhysicalFileSystem, PrintOptions,
    ProfileId, print_diagnostics as print_source_diagnostics,
};

use crate::repository::{
    AmbientSnapshot, Builtins, FileContentId, FileContentStore, Ref, RepositoryError, Revision,
    RevisionState, SourceMap,
};
use crate::{TsConfigOptions, Workspace, WorkspaceKind, resolve_cache_root};
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
    /// The workspace root directory.
    pub(crate) root: PathBuf,

    /// The named movable refs.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// The published source revision graph.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionState>>,
    /// The retained anonymous revision pins.
    pub(crate) pinned_revisions: DashMap<Revision, usize>,

    /// Shared string pool.
    pub strings: Arc<StringPool>,
    /// Shared persistent cache backend.
    pub(crate) cache: Arc<dyn CacheStore>,
    /// The file system backing repository discovery and loads.
    pub(crate) fs: Arc<dyn FileSystem>,
    /// Shared immutable source contents.
    pub(crate) files: FileContentStore,
    /// Shared immutable derived artifacts.
    pub(crate) artifacts: Arc<ArtifactStore>,
    /// Shared builtin selection metadata.
    pub builtins: Arc<Builtins>,
}

impl Repository {
    /// Open one repository at one directory with default options and disk cache.
    pub fn open_root(root: PathBuf, ambient: AmbientSnapshot) -> Self {
        let repository = Self::new(
            root.clone(),
            Arc::new(DiskCacheStore::new()),
            Arc::new(PhysicalFileSystem::new()),
            ambient,
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
        ambient: AmbientSnapshot,
    ) -> Result<Self, RepositoryError> {
        let repository = Self::new(root.clone(), Arc::new(DiskCacheStore::new()), fs, ambient);
        let reference = Ref::for_workspace_root(&root);
        let _ = repository.seed_root_source(&reference)?;
        let _ = repository
            .builtins
            .install_in_repository(&repository, &reference)?;

        Ok(repository)
    }

    /// Create one repository from explicit parts.
    pub fn new(
        root: PathBuf,
        cache: Arc<dyn CacheStore>,
        fs: Arc<dyn FileSystem>,
        ambient: AmbientSnapshot,
    ) -> Self {
        let strings = Arc::new(StringPool::new());
        let files = FileContentStore::new();
        let mut builtins = Builtins::empty();

        let revisions = DashMap::new();
        let refs = DashMap::new();
        let pinned_revisions = DashMap::new();
        let workspace_reference = Ref::for_workspace_root(&root);

        // builtin metadata
        builtins.load_intrinsics();

        let repository = Self {
            root,
            fs,
            strings,
            revisions,
            refs,
            files,
            pinned_revisions,
            artifacts: Arc::new(ArtifactStore::default()),
            builtins: Arc::new(builtins),
            cache,
        };

        // initial repository revision
        let initial_revision = Arc::new(RevisionState::new(
            smallvec::SmallVec::new(),
            Arc::new(SourceMap::new()),
            Arc::new(ambient),
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

    /// Override the backing cache store.
    pub fn with_cache(mut self, cache: Arc<dyn CacheStore>) -> Self {
        self.cache = cache;
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
    pub fn cache(&self) -> &Arc<dyn CacheStore> {
        &self.cache
    }

    /// Print one diagnostic collection with revision scoped source context.
    pub fn print_diagnostics(
        &self,
        revision: Revision,
        diagnostics: &DiagnosticCollection,
        line_width: u32,
    ) {
        let options = PrintOptions::new()
            .with_line_width(line_width)
            .with_module_count(
                self.workspace_module_ids(revision)
                    .unwrap_or_default()
                    .len(),
            );
        let file_for_id = |file_id| {
            self.file(revision, file_id).unwrap_or_else(|error| {
                panic!("failed to load diagnostic file {file_id:?}: {error}")
            })
        };

        print_source_diagnostics(&file_for_id, diagnostics, options);
    }

    /// Return the repository workspace root.
    pub fn workspace_root(&self) -> &Path {
        &self.root
    }

    /// Return one cached workspace snapshot for one revision.
    pub fn workspace(&self, revision: Revision) -> Result<Arc<Workspace>, RepositoryError> {
        let revision_state = self.revision(revision)?;

        if let Some(workspace) = revision_state.workspace.get() {
            return Ok(Arc::clone(workspace));
        }

        let workspace_declaration = self.workspace_destack_declaration(revision)?;
        let packages =
            self.package_snapshots_from_source_map(revision, revision_state.source.as_ref())?;
        let package_paths = self.package_paths(&packages);
        let modules = self.module_snapshots_from_source_map(
            revision,
            revision_state.source.as_ref(),
            &packages,
        )?;
        let kind = if packages.len() > 1 {
            WorkspaceKind::Monorepo
        } else {
            WorkspaceKind::SinglePackage
        };

        let workspace = Arc::new(Workspace {
            file_id: workspace_declaration
                .as_ref()
                .map(|declaration| declaration.file_id),
            root: self.root.clone(),
            kind,
            packages,
            modules,
            package_paths,
            targets: Default::default(),
        });

        let _ = revision_state.workspace.set(Arc::clone(&workspace));

        Ok(revision_state
            .workspace
            .get()
            .map(Arc::clone)
            .unwrap_or(workspace))
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

        ArtifactCache::new(self.cache().as_ref(), &layout)
    }

    /// Resolve one cache root from repository runtime options.
    fn resolve_cache_root(&self) -> PathBuf {
        resolve_cache_root(self.workspace_root(), None)
    }

    /// Return the stable profile id for one semantic profile key.
    pub fn profile_id(&self, key: ProfileKey) -> ProfileId {
        ProfileId::new(key.stable_hash())
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
    ) -> Result<Arc<FileContentEntry>, RepositoryError> {
        self.files
            .get(content)
            .ok_or(RepositoryError::MissingContent { content })
    }
}
