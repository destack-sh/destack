use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactStore, ArtifactTable, ArtifactVersion, BuildId,
};
use destack_core::{Blob, StringPool, TreapRoot};
use destack_source::FileSystem;
use rustc_hash::FxBuildHasher;

use crate::repository::{
    EmbeddedBuiltinPackage, Files, Ref, RepositoryError, Revision, RevisionEntry, RevisionState,
};
use crate::{
    ArtifactBindingTable, BlobStore, DestackLayout, Host, Root, RootKind, Settings, artifact,
};

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
use crate::DiskBlobStore;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use crate::MemoryBlobStore;

/// Content-addressed store for revision source state and derived artifacts.
#[derive(Debug)]
pub struct Repository {
    /// The repository root directory.
    pub(crate) root: PathBuf,

    /// Movable refs pointing at revision identities.
    pub(crate) refs: DashMap<Ref, Revision, FxBuildHasher>,
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
    /// Repository-owned source state.
    pub(crate) files: Files,
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
    /// Create one repository from explicit parts.
    pub fn new(root: PathBuf, host: Host, settings: Settings, layout: DestackLayout) -> Self {
        let revisions = DashMap::default();
        let refs = DashMap::default();
        let root_reference = Ref::for_root(&root);

        let storage = match host.blob_store().cloned() {
            Some(blobs) => {
                let artifacts: Arc<dyn ArtifactStore> = Arc::new(artifact::MemoryStore::new());

                Storage { blobs, artifacts }
            }
            None => Storage::open(&root, &layout, host.build_id()),
        };

        let repository = Self {
            root,
            host,
            revisions,
            refs,
            files: Files::new(),
            artifact_table: Arc::new(ArtifactTable::default()),
            artifact_store: storage.artifacts,
            pending_artifacts: DashMap::default(),
            blobs: storage.blobs,
            mounts: DashMap::default(),
            embedded_builtin: EmbeddedBuiltinPackage::new(),
            strings: Arc::new(StringPool::new()),
            layout,
            settings,
        };

        // create initial repository revision
        let initial_revision = Arc::new(RevisionState::new(
            TreapRoot::new(),
            Arc::new(repository.host.environment().clone()),
            ArtifactBindingTable::new(),
        ));
        let initial_revision_id = initial_revision.revision();
        repository.revisions.insert(
            initial_revision_id,
            Arc::new(RevisionEntry::new(initial_revision)),
        );
        repository.refs.insert(root_reference, initial_revision_id);

        repository
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

    /// Store exact immutable bytes.
    pub fn put_blob(&self, bytes: &[u8]) -> Result<Blob, RepositoryError> {
        let mut input = Cursor::new(bytes);
        self.blobs
            .put(&mut input)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Require one exact Blob in repository storage.
    pub(crate) fn require_blob(&self, blob: Blob) -> Result<(), RepositoryError> {
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
    /// Open disk storage for one Repository.
    fn open(root: &Path, layout: &DestackLayout, build_id: BuildId) -> Self {
        let blobs: Arc<dyn BlobStore> = Arc::new(DiskBlobStore::new(layout.blob_directory()));
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
    /// Open memory storage for one Repository.
    fn open(_root: &Path, _layout: &DestackLayout, _build_id: BuildId) -> Self {
        let blobs = Arc::new(MemoryBlobStore::new());
        let artifacts = Arc::new(artifact::MemoryStore::new());

        Self { blobs, artifacts }
    }
}
