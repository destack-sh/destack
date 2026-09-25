use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use rustc_hash::FxBuildHasher;
use tspp_artifact::{ArtifactCache, ArtifactTable, BuildId};
use tspp_core::{Blob, BlobMemory, BlobStore, StringPool, Treap, TreapRoot};
use tspp_source as source;
use tspp_source::{FileId, FileSystem, MemoryFileSystem};

use crate::repository::{
    FileCache, FileEntry, RepositoryError, Revision, RevisionEntry, RevisionState,
};
use crate::{
    ArtifactSelection, DestackLayout, DestackLayoutOverride, Environment, Host, Root, RootKind,
    Settings, SourceRoot,
};

/// Revisioned source repository backed by shared host state.
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
    /// Persistent file entry tree.
    pub(crate) file_tree: Treap<FileId, FileEntry>,
    /// Loaded source state caches.
    pub(crate) file_cache: FileCache,
    /// Named physical bases for dependency roots outside the workspace.
    pub(crate) mounts: DashMap<String, PathBuf, FxBuildHasher>,
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
        build_id: BuildId,
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
        let host = Host::new(build_id, environment, file_system);
        Self::open(root, host, settings, layout_override)
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
            ArtifactSelection::new(),
        ));
        let empty_revision = empty.revision();
        revisions.insert(empty_revision, Arc::new(RevisionEntry::new(empty)));

        let repository = Self {
            root,
            revisions,
            host,
            layout,
            settings,
            file_tree: Treap::new(),
            file_cache: FileCache::default(),
            mounts: DashMap::default(),
        };

        (repository, empty_revision)
    }

    /// Return the repository file system.
    pub fn file_system(&self) -> &Arc<dyn FileSystem> {
        self.host.files()
    }

    /// Return the repository artifact table.
    pub fn artifact_table(&self) -> &Arc<ArtifactTable> {
        self.host.artifact_table()
    }

    /// Return persistent artifact storage when configured.
    pub fn artifact_cache(&self) -> Option<&ArtifactCache> {
        self.host.artifact_cache()
    }

    /// Return the repository string pool.
    pub fn string_pool(&self) -> &Arc<StringPool> {
        self.host.string_pool()
    }

    /// Return the repository blob store.
    pub fn blob_store(&self) -> &Arc<BlobStore> {
        self.host.blob_store()
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

        // classify the repository root
        let root_config = self.manifest_for_workspace(revision)?;
        let kind = match root_config.as_ref() {
            Some(config) if config.workspaces.is_some() => RootKind::Workspace,
            Some(_) => RootKind::Package,
            None => RootKind::Loose,
        };

        // retain root metadata for this revision
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
        self.layout.cache.clone()
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

    /// Retain exact immutable bytes on this repository's host.
    pub fn retain_blob(&self, bytes: &[u8]) -> Result<Blob, RepositoryError> {
        self.blob_store()
            .retain_bytes(bytes)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Return whether one exact Blob is available from this repository.
    pub fn contains_blob(&self, blob: Blob) -> Result<bool, RepositoryError> {
        self.blob_store()
            .contains(blob)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Open one exact Blob.
    pub fn open_blob(&self, blob: Blob) -> Result<Arc<BlobMemory>, RepositoryError> {
        self.blob_store()
            .open(blob)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })
    }

    /// Require one exact Blob.
    pub(crate) fn require_blob(&self, blob: Blob) -> Result<(), RepositoryError> {
        if !self.contains_blob(blob)? {
            return Err(RepositoryError::Blob {
                message: format!("missing Blob {blob}"),
            });
        }

        Ok(())
    }
}
