use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactImageCache, ArtifactImageCacheLayout, ArtifactKey, ArtifactStore, ArtifactVersion,
    CacheStore,
};
use destack_core::StringPool;
use destack_source::{FileContentEntry, FileContentId, FileId, FileSystem};
use im::OrdMap;

use crate::repository::{
    FileCache, FileEntry, FileStore, HostEnvironment, Ref, RepositoryError, Revision,
    RevisionCache, RevisionState,
};
use crate::{Workspace, WorkspaceKind, resolve_cache_root};

/// Content-addressed store for revision source state and derived artifacts.
#[derive(Debug)]
pub struct Repository {
    /// The workspace root directory.
    pub(crate) root: PathBuf,

    /// Movable refs pointing at revision identities.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// Immutable source states keyed by revision identity.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionState>>,
    /// Active anonymous revision retain counts.
    pub(crate) revision_pins: DashMap<Revision, usize>,
    /// Derived indexes keyed by revision identity.
    pub(crate) revision_caches: DashMap<Revision, Arc<RevisionCache>>,
    /// Exact artifact versions bound to revision-local artifact keys.
    pub(crate) artifact_versions: DashMap<(Revision, ArtifactKey), ArtifactVersion>,

    /// Shared persistent cache backend.
    pub(crate) cache: Arc<dyn CacheStore>,
    /// The file system backing repository discovery and loads.
    pub(crate) fs: Arc<dyn FileSystem>,
    /// Shared immutable file contents.
    pub(crate) files: FileStore,
    /// Parsed file data by exact file content.
    pub(crate) file_cache: FileCache,
    /// Shared derived artifacts.
    pub(crate) artifacts: Arc<ArtifactStore>,
    /// Shared interned strings for this repository.
    pub(crate) strings: Arc<StringPool>,
}

impl Repository {
    /// Create one repository from explicit parts.
    pub fn new(
        root: PathBuf,
        cache: Arc<dyn CacheStore>,
        fs: Arc<dyn FileSystem>,
        host: HostEnvironment,
    ) -> Self {
        let file_contents = FileStore::new();

        let revisions = DashMap::new();
        let refs = DashMap::new();
        let revision_pins = DashMap::new();
        let artifact_versions = DashMap::new();
        let revision_caches = DashMap::new();
        let file_cache = FileCache::new();
        let workspace_reference = Ref::for_workspace_root(&root);

        let repository = Self {
            root,
            fs,
            revisions,
            refs,
            artifact_versions,
            revision_caches,
            file_cache,
            files: file_contents,
            revision_pins,
            artifacts: Arc::new(ArtifactStore::default()),
            strings: Arc::new(StringPool::new()),
            cache,
        };

        // initial repository revision
        let initial_revision = Arc::new(RevisionState::new(
            Arc::new(OrdMap::<FileId, FileEntry>::new()),
            Arc::new(host),
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

    /// Return the repository artifact store.
    pub fn artifact_store(&self) -> &Arc<ArtifactStore> {
        &self.artifacts
    }

    /// Return the repository string pool.
    pub fn string_pool(&self) -> &Arc<StringPool> {
        &self.strings
    }

    /// Return the repository cache store.
    pub fn cache(&self) -> &Arc<dyn CacheStore> {
        &self.cache
    }

    /// Return the repository workspace root.
    pub fn workspace_root(&self) -> &Path {
        &self.root
    }

    /// Return workspace metadata for one revision.
    pub fn workspace(&self, revision: Revision) -> Result<Arc<Workspace>, RepositoryError> {
        let _revision_state = self.revision(revision)?;
        let revision_cache = self.revision_cache(revision);

        if let Some(workspace) = revision_cache.workspace.get() {
            return Ok(Arc::clone(workspace));
        }

        let workspace_config = self.destack_config_for_workspace(revision)?;
        let packages = self.package_index(revision)?;
        let kind = if packages.len() > 1 {
            WorkspaceKind::Monorepo
        } else {
            WorkspaceKind::SinglePackage
        };

        let workspace = Arc::new(Workspace {
            file_id: workspace_config.as_ref().map(|config| config.file_id),
            root: self.root.clone(),
            kind,
        });

        let workspace = revision_cache.workspace.get_or_init(|| workspace);

        Ok(Arc::clone(workspace))
    }

    /// Resolve the repository cache directory.
    pub fn cache_directory(&self) -> PathBuf {
        self.resolve_cache_root()
    }

    /// Build one persisted artifact cache layout.
    pub fn artifact_image_cache_layout(&self, cache_abi: &str) -> ArtifactImageCacheLayout {
        let cache_root = self.resolve_cache_root();
        let is_shared_root = !cache_root.starts_with(self.workspace_root());

        ArtifactImageCacheLayout::new(
            &cache_root,
            self.workspace_root(),
            cache_abi,
            is_shared_root,
        )
    }

    /// Build one persisted artifact cache.
    pub fn artifact_image_cache(&self, cache_abi: &str) -> ArtifactImageCache<'_> {
        let layout = self.artifact_image_cache_layout(cache_abi);

        ArtifactImageCache::new(self.cache().as_ref(), &layout)
    }

    /// Resolve one cache root from repository runtime options.
    fn resolve_cache_root(&self) -> PathBuf {
        resolve_cache_root(self.workspace_root(), None)
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

    /// Return one immutable revision state.
    pub(crate) fn revision(
        &self,
        revision: Revision,
    ) -> Result<Arc<RevisionState>, RepositoryError> {
        self.revisions
            .get(&revision)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(RepositoryError::MissingRevision { revision })
    }

    /// Return the derived cache for one revision.
    pub(crate) fn revision_cache(&self, revision: Revision) -> Arc<RevisionCache> {
        self.revision_caches
            .entry(revision)
            .or_insert_with(|| Arc::new(RevisionCache::new()))
            .clone()
    }

    /// Return one shared file content payload by exact content id.
    pub(crate) fn file_content_by_id(
        &self,
        content: FileContentId,
    ) -> Result<Arc<FileContentEntry>, RepositoryError> {
        self.files
            .get(content)
            .ok_or(RepositoryError::MissingContent { content })
    }
}
