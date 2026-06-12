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
    BuiltinPackage, FileCache, FileEntry, FileStore, Ref, RepositoryError, Revision, RevisionEntry,
    RevisionState,
};
use crate::{DestackLayout, Environment, Root, RootKind, Settings};

/// Content-addressed store for revision source state and derived artifacts.
#[derive(Debug)]
pub struct Repository {
    /// The repository root directory.
    pub(crate) root: PathBuf,

    /// Movable refs pointing at revision identities.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// Immutable source states keyed by revision identity.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionEntry>>,
    /// Exact artifact versions bound to revision-local artifact keys.
    pub(crate) artifact_versions: DashMap<(Revision, ArtifactKey), ArtifactVersion>,

    /// Shared persistent cache backend.
    pub(crate) cache: Arc<dyn CacheStore>,
    /// Resolved storage layout for this repository.
    pub(crate) layout: DestackLayout,
    /// Machine-local settings used to open this repository.
    pub(crate) settings: Settings,
    /// The file system backing repository discovery and loads.
    pub(crate) fs: Arc<dyn FileSystem>,
    /// Shared immutable file contents.
    pub(crate) files: FileStore,
    /// Immutable builtin package shipped with the toolchain.
    pub(crate) builtin: BuiltinPackage,
    /// Parsed file data by exact file content.
    pub(crate) file_cache: FileCache,
    /// Named physical bases for dependency roots outside the workspace.
    pub(crate) mounts: DashMap<String, PathBuf>,
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
        environment: Environment,
        settings: Settings,
        layout: DestackLayout,
    ) -> Self {
        let file_contents = FileStore::new();

        let revisions = DashMap::new();
        let refs = DashMap::new();
        let artifact_versions = DashMap::new();
        let file_cache = FileCache::new();
        let root_reference = Ref::for_root(&root);

        let repository = Self {
            root,
            fs,
            revisions,
            refs,
            artifact_versions,
            file_cache,
            files: file_contents,
            mounts: DashMap::new(),
            builtin: BuiltinPackage::new(),
            artifacts: Arc::new(ArtifactStore::default()),
            strings: Arc::new(StringPool::new()),
            cache,
            layout,
            settings,
        };

        // create initial repository revision
        let initial_revision = Arc::new(RevisionState::new(
            Arc::new(OrdMap::<FileId, FileEntry>::new()),
            Arc::new(environment),
        ));
        let initial_revision_id = initial_revision.revision();
        repository.revisions.insert(
            initial_revision_id,
            Arc::new(RevisionEntry::new(initial_revision)),
        );
        repository.refs.insert(root_reference, initial_revision_id);

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

    /// Return the resolved repository layout.
    pub fn layout(&self) -> &DestackLayout {
        &self.layout
    }

    /// Return the loaded machine-local settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Return the repository root path.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Return root metadata for one revision.
    pub fn root(&self, revision: Revision) -> Result<Arc<Root>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

        if let Some(root) = revision_cache.root.get() {
            return Ok(Arc::clone(root));
        }

        let root_config = self.destack_for_workspace(revision)?;
        let packages = self.package_index(revision)?;
        let kind = if packages.len() > 1 {
            RootKind::Monorepo
        } else {
            RootKind::SinglePackage
        };

        let root = Arc::new(Root {
            file_id: root_config.as_ref().map(|config| config.file_id),
            root: self.root.clone(),
            kind,
        });

        let root = revision_cache.root.get_or_init(|| root);

        Ok(Arc::clone(root))
    }

    /// Resolve the repository cache directory.
    pub fn cache_directory(&self) -> PathBuf {
        self.layout.workspace_cache.clone()
    }

    /// Build one persisted artifact cache layout.
    pub fn artifact_image_cache_layout(&self, cache_abi: &str) -> ArtifactImageCacheLayout {
        let cache_root = self.cache_directory();
        let is_shared_root = !cache_root.starts_with(self.path());

        ArtifactImageCacheLayout::new(&cache_root, self.path(), cache_abi, is_shared_root)
    }

    /// Build one persisted artifact cache.
    pub fn artifact_image_cache(&self, cache_abi: &str) -> ArtifactImageCache<'_> {
        let layout = self.artifact_image_cache_layout(cache_abi);

        ArtifactImageCache::new(self.cache().as_ref(), &layout)
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
            .map(|entry| entry.value().state())
            .ok_or(RepositoryError::MissingRevision { revision })
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
