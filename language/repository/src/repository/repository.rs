use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactStore, ArtifactTable, ArtifactVersion, BlobStore, ContentStore,
    RepositoryStoreLayout, SegmentedArtifactStore,
};
use destack_core::{StringPool, TreapRoot};
use destack_source::{Content, ContentEntry, ContentId, File, FileSystem};

use crate::repository::{
    ContentPool, EmbeddedBuiltinPackage, Files, Ref, RepositoryError, Revision, RevisionEntry,
    RevisionState,
};
use crate::{ArtifactBindingTable, DestackLayout, Host, Root, RootKind, Settings};

/// Content-addressed store for revision source state and derived artifacts.
#[derive(Debug)]
pub struct Repository {
    /// The repository root directory.
    pub(crate) root: PathBuf,

    /// Movable refs pointing at revision identities.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// Immutable source states keyed by revision identity.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionEntry>>,

    /// Host capabilities available to repository tooling.
    pub(crate) host: Host,
    /// Resolved storage layout for this repository.
    pub(crate) layout: DestackLayout,
    /// Machine-local settings used to open this repository.
    pub(crate) settings: Settings,
    /// Shared immutable in-process contents.
    pub(crate) content_pool: ContentPool,
    /// Embedded Builtin Package shipped with the current build.
    pub(crate) embedded_builtin: EmbeddedBuiltinPackage,
    /// Repository-owned source state.
    pub(crate) files: Files,
    /// Shared typed derived artifacts.
    pub(crate) artifact_table: Arc<ArtifactTable>,
    /// Persistent artifact records.
    pub(crate) artifact_store: Arc<dyn ArtifactStore>,
    /// Dependency observations needed to persist completed artifact versions.
    pub(crate) pending_artifacts: DashMap<ArtifactVersion, Arc<[ArtifactDependency]>>,
    /// Named physical bases for dependency roots outside the workspace.
    pub(crate) mounts: DashMap<String, PathBuf>,
    /// Shared interned strings for this repository.
    pub(crate) strings: Arc<StringPool>,
}

impl Repository {
    /// Create one repository from explicit parts.
    pub fn new(root: PathBuf, host: Host, settings: Settings, layout: DestackLayout) -> Self {
        let content_pool = ContentPool::new();

        let revisions = DashMap::new();
        let refs = DashMap::new();
        let root_reference = Ref::for_root(&root);

        let artifact_store = Arc::new(SegmentedArtifactStore::new(
            Arc::clone(host.blob_store()),
            Self::repository_store_layout_for(&root, &layout),
            host.build_id(),
        ));

        let repository = Self {
            root,
            host,
            revisions,
            refs,
            files: Files::new(),
            artifact_table: Arc::new(ArtifactTable::default()),
            artifact_store,
            pending_artifacts: DashMap::new(),
            content_pool,
            mounts: DashMap::new(),
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

    /// Override the backing blob store.
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        let artifact_store = Arc::new(SegmentedArtifactStore::new(
            Arc::clone(&blob_store),
            self.repository_store_layout(),
            self.host.build_id(),
        ));

        self.host.set_blob_store(blob_store);
        self.artifact_store = artifact_store;

        self
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
            return Ok(Arc::clone(root));
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

        Ok(Arc::clone(root))
    }

    /// Resolve the repository cache directory.
    pub fn cache_directory(&self) -> PathBuf {
        self.layout.workspace_cache.clone()
    }

    /// Build one persisted repository store layout.
    pub fn repository_store_layout(&self) -> RepositoryStoreLayout {
        Self::repository_store_layout_for(self.path(), self.layout())
    }

    /// Build one persisted repository store layout from explicit parts.
    fn repository_store_layout_for(root: &Path, layout: &DestackLayout) -> RepositoryStoreLayout {
        let cache_root = layout.workspace_cache.clone();
        let is_shared_root = !cache_root.starts_with(root);

        RepositoryStoreLayout::new(&cache_root, root, is_shared_root)
    }

    /// Build one persisted content store.
    pub fn content_store(&self) -> ContentStore<'_> {
        let layout = self.repository_store_layout();

        ContentStore::new(self.blob_store().as_ref(), &layout)
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

    /// Intern one immutable content payload.
    pub fn intern_content(&self, content: Content) -> Result<ContentId, RepositoryError> {
        let length = content.byte_length();
        if length > File::MAX_BYTES {
            return Err(RepositoryError::ContentTooLarge { length });
        }

        // persist and intern content
        let content_id = self.content_store().store(&content).map_err(|error| {
            RepositoryError::ContentStore {
                message: error.to_string(),
            }
        })?;
        let _entry = self.content_pool.insert(content_id, content);

        Ok(content_id)
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, content: ContentId) -> Result<Arc<ContentEntry>, RepositoryError> {
        if let Some(entry) = self.content_pool.get(content) {
            return Ok(entry);
        }

        let Some(payload) =
            self.content_store()
                .load(content)
                .map_err(|error| RepositoryError::ContentStore {
                    message: error.to_string(),
                })?
        else {
            return Err(RepositoryError::MissingContent { content });
        };

        // validate stored content
        let length = payload.byte_length();
        if length > File::MAX_BYTES {
            return Err(RepositoryError::ContentTooLarge { length });
        }

        // intern the loaded content
        let entry = self.content_pool.insert(content, payload);

        Ok(entry)
    }
}
