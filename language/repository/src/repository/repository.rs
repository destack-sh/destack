use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactStore, ArtifactTable, ArtifactVersion, BuildId,
};
use destack_core::{Blob, BlobMemory, StringPool, Treap, TreapRoot};
use destack_source as source;
use destack_source::{FileId, FileSystem, MemoryFileSystem};
use rustc_hash::FxBuildHasher;

use crate::repository::{
    EmbeddedBuiltinPackage, FileCache, FileEntry, RepositoryError, Revision, RevisionEntry,
    RevisionState,
};
use crate::{
    ArtifactBindingTable, BlobStore, DestackLayout, DestackLayoutOverride, Environment, Host,
    MemoryBlobStore, Root, RootKind, Settings, SourceRoot, artifact,
};

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
use crate::DiskBlobStore;

/// Content-addressed store for revision source state and derived artifacts.
#[derive(Debug)]
pub struct Repository {
    /// The repository root directory.
    pub(crate) root: PathBuf,

    /// Immutable source states keyed by revision identity.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionEntry>, FxBuildHasher>,

    /// Host capabilities available to repository tooling.
    pub(crate) host: Host,
    /// Resolved storage layout for this repository.
    pub(crate) layout: DestackLayout,
    /// Machine-local settings used to open this repository.
    pub(crate) settings: Settings,
    /// Shared immutable Blob storage.
    pub(crate) blobs: Arc<dyn BlobStore>,
    /// Embedded Builtin Package shipped with the current build.
    pub(crate) embedded_builtin: EmbeddedBuiltinPackage,
    /// Persistent file entry tree.
    pub(crate) file_tree: Treap<FileId, FileEntry>,
    /// Loaded source state caches.
    pub(crate) file_cache: FileCache,
    /// Shared typed derived artifacts.
    pub(crate) artifact_table: Arc<ArtifactTable>,
    /// Persistent artifact records.
    pub(crate) artifact_store: Arc<dyn ArtifactStore>,
    /// Dependency observations needed to persist completed artifact versions.
    pub(crate) pending_artifacts:
        DashMap<ArtifactVersion, Arc<[ArtifactDependency]>, FxBuildHasher>,
    /// Named physical bases for dependency roots outside the workspace.
    pub(crate) mounts: DashMap<String, PathBuf, FxBuildHasher>,
    /// Shared interned strings for this repository.
    pub(crate) strings: Arc<StringPool>,
}

/// Physical storage selected for one Repository.
struct Storage {
    /// Immutable Blob storage.
    blobs: Arc<dyn BlobStore>,
    /// Persistent artifact record storage.
    artifacts: Arc<dyn ArtifactStore>,
}

impl Repository {
    /// Open one repository and return its imported physical revision.
    pub fn open(
        path: PathBuf,
        host: Host,
        settings: Settings,
        layout_override: DestackLayoutOverride,
    ) -> Result<(Self, Revision), RepositoryError> {
        let root = PathBuf::from(SourceRoot::discover(host.files().as_ref(), &path)?);
        let environment = host.environment();
        let cwd = environment.cwd.as_deref().unwrap_or(&path);
        let layout =
            DestackLayout::resolve(&root, cwd, environment, &settings, &layout_override, None);

        // create repository at the selected source root
        let (repository, base) = Self::new(root.clone(), host, settings, layout);

        // import the complete physical tree
        let edits = repository.scan(&root, base)?;
        let revision = repository.edit(base, edits)?.after;

        Ok((repository, revision))
    }

    /// Open one repository and return its imported in-memory revision.
    pub fn memory(
        root: PathBuf,
        edits: Vec<source::Edit>,
        environment: Environment,
        settings: Settings,
        layout_override: DestackLayoutOverride,
    ) -> Result<(Self, Revision), RepositoryError> {
        let file_system = Arc::new(MemoryFileSystem::new());
        file_system
            .create_dir_all(&root)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "create_dir_all",
                path: root.clone(),
                message: error.to_string(),
            })?;

        // materialize the supplied physical source state
        for edit in edits {
            let path = edit
                .path()
                .map(|path| root.join(path))
                .unwrap_or_else(|| root.clone());
            edit.apply(&root, file_system.as_ref()).map_err(|error| {
                RepositoryError::FileSystem {
                    operation: "apply",
                    path,
                    message: error.to_string(),
                }
            })?;
        }

        // keep source Blobs and derived artifacts in memory
        let build_id = BuildId::current().map_err(|error| RepositoryError::ArtifactStore {
            message: format!("failed to identify Destack build: {error}"),
        })?;
        let blobs: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::new());
        let host = Host::new(build_id, environment, file_system).with_blob_store(blobs);
        let (repository, revision) = Self::open(root, host, settings, layout_override)?;
        let repository = repository.with_artifact_store(Arc::new(artifact::MemoryStore::new()));

        Ok((repository, revision))
    }

    /// Create one empty repository and return its empty revision.
    pub fn new(
        root: PathBuf,
        host: Host,
        settings: Settings,
        layout: DestackLayout,
    ) -> (Self, Revision) {
        // create one immutable empty revision
        let revisions = DashMap::default();
        let empty = Arc::new(RevisionState::new(
            TreapRoot::new(),
            Arc::new(host.environment().clone()),
            ArtifactBindingTable::new(),
        ));
        let empty_revision = empty.revision();
        revisions.insert(empty_revision, Arc::new(RevisionEntry::new(empty)));

        // open durable storage selected by this repository
        let storage = Storage::open(&root, &layout, host.build_id(), host.blob_store().cloned());

        let repository = Self {
            root,
            revisions,
            host,
            layout,
            settings,
            blobs: storage.blobs,
            embedded_builtin: EmbeddedBuiltinPackage::new(),
            file_tree: Treap::new(),
            file_cache: FileCache::default(),
            artifact_table: Arc::new(ArtifactTable::default()),
            artifact_store: storage.artifacts,
            pending_artifacts: DashMap::default(),
            mounts: DashMap::default(),
            strings: Arc::new(StringPool::new()),
        };

        (repository, empty_revision)
    }

    /// Override the persistent artifact store.
    pub fn with_artifact_store(mut self, artifact_store: Arc<dyn ArtifactStore>) -> Self {
        self.artifact_store = artifact_store;
        self
    }

    /// Return the repository file system.
    pub fn file_system(&self) -> &Arc<dyn FileSystem> {
        self.host.files()
    }

    /// Return the repository artifact table.
    pub fn artifact_table(&self) -> &Arc<ArtifactTable> {
        &self.artifact_table
    }

    /// Return the persistent artifact store.
    pub fn artifact_store(&self) -> &Arc<dyn ArtifactStore> {
        &self.artifact_store
    }

    /// Return the repository string pool.
    pub fn string_pool(&self) -> &Arc<StringPool> {
        &self.strings
    }

    /// Return the repository blob store.
    pub fn blob_store(&self) -> &Arc<dyn BlobStore> {
        &self.blobs
    }

    /// Return the repository host capabilities.
    pub fn host(&self) -> &Host {
        &self.host
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
            return Ok(root.clone());
        }

        let root_config = self.destack_for_workspace(revision)?;
        let is_workspace = root_config
            .as_ref()
            .is_some_and(|config| config.workspace_packages().is_some());
        let kind = if is_workspace {
            RootKind::Workspace
        } else {
            RootKind::Package
        };

        let root = Arc::new(Root {
            file_id: root_config.as_ref().map(|config| config.file_id),
            root: self.root.clone(),
            kind,
        });

        let root = revision_cache.root.get_or_init(|| root);

        Ok(root.clone())
    }

    /// Resolve the repository cache directory.
    pub fn cache_directory(&self) -> PathBuf {
        self.layout.workspace_cache.clone()
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

    /// Store exact immutable bytes.
    pub fn put_blob(&self, bytes: &[u8]) -> Result<Blob, RepositoryError> {
        let mut input = Cursor::new(bytes);
        self.blobs
            .put(&mut input)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Open one exact Blob, serving embedded builtin content directly.
    pub fn open_blob(&self, blob: Blob) -> Result<Arc<BlobMemory>, RepositoryError> {
        // serve embedded builtin sources from their retained memory
        if let Some(memory) = self.embedded_builtin.builtin_blob_memory(blob.id) {
            return Ok(memory);
        }

        self.blobs
            .open(blob)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Require one exact Blob, counting embedded builtin content as present.
    pub(crate) fn require_blob(&self, blob: Blob) -> Result<(), RepositoryError> {
        // embedded builtin sources are always present in their retained memory
        if self.embedded_builtin.builtin_blob_memory(blob.id).is_some() {
            return Ok(());
        }

        let is_present = self
            .blobs
            .contains(blob)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })?;
        if !is_present {
            return Err(RepositoryError::Blob {
                message: format!("missing Blob {blob}"),
            });
        }

        Ok(())
    }
}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
impl Storage {
    /// Open storage for one native Repository.
    fn open(
        root: &Path,
        layout: &DestackLayout,
        build_id: BuildId,
        blobs: Option<Arc<dyn BlobStore>>,
    ) -> Self {
        let blobs = blobs.unwrap_or_else(|| Arc::new(DiskBlobStore::new(layout.blob_directory())));
        let artifacts = Arc::new(artifact::DiskStore::new(
            root,
            layout,
            build_id,
            blobs.clone(),
        ));

        Self { blobs, artifacts }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Storage {
    /// Open storage for one WebAssembly Repository.
    fn open(
        _root: &Path,
        _layout: &DestackLayout,
        _build_id: BuildId,
        blobs: Option<Arc<dyn BlobStore>>,
    ) -> Self {
        let blobs = blobs.unwrap_or_else(|| Arc::new(MemoryBlobStore::new()));
        let artifacts = Arc::new(artifact::MemoryStore::new());

        Self { blobs, artifacts }
    }
}
