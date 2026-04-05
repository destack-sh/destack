use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_source::{
    Diagnostic, File, FileContent, FileId, FileSystem, ModuleId, OverlayFileSystem, Uri,
};
use destack_workspace::{Change, Edit, Ref, Repository, RepositorySnapshot, Revision};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    FileSnapshot, FileUpdate, LanguageServiceError, UpdateImpact, WorkspaceMessage,
    WorkspaceMessageKind, WorkspaceUpdateRecord,
};

/// One tracked open-document identity owned by one workspace root.
#[derive(Debug, Clone)]
pub(super) struct TrackedDocumentState {
    /// The current document path.
    pub(super) path: PathBuf,
    /// The current document uri.
    pub(super) uri: Uri,
    /// The current client document version.
    pub(super) version: i32,
}

/// Build a stable tracked-document key for one path.
pub(super) fn tracked_document_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Per workspace root live session state.
#[derive(Debug)]
pub(super) struct WorkspaceSession {
    /// Root path for this workspace.
    pub(super) root: PathBuf,
    /// Repository that owns this workspace root.
    repository: Arc<Repository>,
    /// Moving repository ref for this workspace root.
    revision_ref: Ref,
    /// Overlay filesystem projection for tracked document content.
    overlay_fs: Option<Arc<OverlayFileSystem>>,
    /// Compiler for this root.
    compiler: Arc<Compiler>,
    /// Tracked open-document state keyed by normalized path.
    tracked_documents_by_path: RwLock<Arc<HashMap<PathBuf, TrackedDocumentState>>>,
    /// Current diagnostics grouped by file for this root.
    diagnostics_by_file: RwLock<Arc<BTreeMap<FileId, Vec<Diagnostic>>>>,
    /// Serialize semantic access per root.
    pub(super) compile_lock: RwLock<()>,
}

impl WorkspaceSession {
    /// Create a new workspace session for a root.
    pub(super) fn new(
        root: PathBuf,
        repository: Arc<Repository>,
        revision_ref: Ref,
        overlay_fs: Option<Arc<OverlayFileSystem>>,
        compiler: Arc<Compiler>,
    ) -> Self {
        Self {
            root,
            repository,
            revision_ref,
            overlay_fs,
            compiler,
            tracked_documents_by_path: RwLock::new(Arc::new(HashMap::new())),
            diagnostics_by_file: RwLock::new(Arc::new(BTreeMap::new())),
            compile_lock: RwLock::new(()),
        }
    }

    /// Return the current semantic revision.
    pub(super) fn revision(&self) -> Revision {
        self.repository
            .current(&self.revision_ref)
            .expect("workspace ref should be tracked")
    }

    /// Return the current pinned semantic snapshot.
    pub(super) fn snapshot(&self) -> RepositorySnapshot {
        let revision = self.revision();

        self.repository
            .snapshot(revision)
            .expect("workspace ref should resolve to a pinned snapshot")
    }

    /// Import the current on-disk file state for this workspace root into the workspace ref.
    pub(super) fn sync_file_system(&self) -> Result<Revision, LanguageServiceError> {
        self.repository
            .import_from_fs(&self.revision_ref, &self.root)
            .map_err(LanguageServiceError::from)
    }

    /// Stage one file update against the current workspace revision.
    pub(super) fn stage_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<Revision, LanguageServiceError> {
        let logical_path = self.repository.normalize_workspace_path(path);
        let change = match update {
            FileUpdate::Text { content } => Change::from(Edit::set_text(logical_path, content)),
            FileUpdate::Bytes { content } => Change::from(Edit::SetFile {
                logical_path,
                content: FileContent::Binary { content },
            }),
            FileUpdate::Removed => Change::from(Edit::remove_file(logical_path)),
        };

        self.repository
            .apply_to_revision(self.revision(), change)
            .map_err(LanguageServiceError::from)
    }

    /// Publish one staged revision as the current workspace revision.
    pub(super) fn publish_revision(
        &self,
        revision: Revision,
    ) -> Result<Revision, LanguageServiceError> {
        self.repository
            .point(&self.revision_ref, revision)
            .map_err(LanguageServiceError::from)
    }

    /// Apply one file update to the tracked workspace ref.
    pub(super) fn apply_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<Revision, LanguageServiceError> {
        let revision = self.stage_file_update(path, update)?;
        self.publish_revision(revision)
    }

    /// Return the current repository for this workspace.
    pub(super) fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the compiler for this workspace.
    pub(super) fn compiler(&self) -> Arc<Compiler> {
        self.compiler.clone()
    }

    /// Resolve one repository file id for a path using both raw and canonical keys.
    pub(super) fn resolve_file_id_for_path(&self, path: &Path) -> Option<FileId> {
        let revision = self.revision();
        let file_id = self.repository.file_id_for_workspace_path(path);
        if self
            .repository
            .file(revision, file_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(file_id);
        }

        let canonical_path = tracked_document_key(path);
        if canonical_path != path {
            let file_id = self.repository.file_id_for_workspace_path(&canonical_path);
            if self
                .repository
                .file(revision, file_id)
                .ok()
                .flatten()
                .is_some()
            {
                return Some(file_id);
            }
        }

        None
    }

    /// Return true when the current semantic workspace contains this path.
    pub(super) fn owns_semantic_path(&self, path: &Path) -> bool {
        self.resolve_file_id_for_path(path).is_some()
    }

    /// Return one tracked file snapshot for a path when present.
    pub(super) fn file_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<(FileId, Arc<File>)>, LanguageServiceError> {
        let Some(file_id) = self.resolve_file_id_for_path(path) else {
            return Ok(None);
        };
        let file = self
            .repository
            .file(self.revision(), file_id)
            .map_err(LanguageServiceError::from)?
            .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;

        Ok(Some((file_id, file)))
    }

    /// Return one tracked file snapshot by id.
    pub(super) fn file_for_id(&self, file_id: FileId) -> Result<Arc<File>, LanguageServiceError> {
        let file = self
            .repository
            .file(self.revision(), file_id)
            .map_err(LanguageServiceError::from)?
            .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;

        Ok(file)
    }

    /// Return current diagnostics for one file.
    pub(super) fn diagnostics_for_file(&self, file_id: FileId) -> Vec<Diagnostic> {
        self.diagnostics_by_file
            .read()
            .get(&file_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Return current diagnostics grouped by file.
    pub(super) fn diagnostics_by_file(&self) -> Vec<(FileId, Vec<Diagnostic>)> {
        self.diagnostics_by_file
            .read()
            .iter()
            .map(|(file_id, diagnostics)| (*file_id, diagnostics.clone()))
            .collect()
    }

    /// Replace diagnostics for updated files at one revision.
    pub(super) fn apply_diagnostics(&self, updates: Vec<(FileId, Vec<Diagnostic>)>) {
        let current = self.diagnostics_by_file.read().clone();
        let mut diagnostics_by_file = (*current).clone();

        for (file_id, diagnostics) in updates {
            diagnostics_by_file.insert(file_id, diagnostics);
        }

        *self.diagnostics_by_file.write() = Arc::new(diagnostics_by_file);
    }

    /// Build one publish identity for a file snapshot.
    fn publish_identity_for_file(
        &self,
        file: &FileSnapshot,
        preferred_uri: Option<&Uri>,
    ) -> (Uri, Option<i32>) {
        let document_identity = file
            .path
            .as_ref()
            .and_then(|path| self.tracked_document_identity_for_path(path));

        let publish_uri = document_identity
            .as_ref()
            .map(|(uri, _)| uri.clone())
            .or_else(|| preferred_uri.cloned())
            .or_else(|| file.path.as_ref().map(Uri::from_file_path))
            .unwrap_or_else(|| file.uri.clone());
        let publish_version = document_identity.as_ref().map(|(_, version)| *version);

        (publish_uri, publish_version)
    }

    /// Build one public workspace update record.
    pub(super) fn workspace_update_record(
        &self,
        update: ServiceUpdate,
        preferred_uri: Option<&Uri>,
    ) -> WorkspaceUpdateRecord {
        let (publish_uri, publish_version) =
            self.publish_identity_for_file(&update.file, preferred_uri);

        WorkspaceUpdateRecord {
            module_id: update.module_id,
            file_id: update.file_id,
            publish_uri,
            publish_version,
            file: update.file,
            impact: update.impact,
            diagnostics: update.diagnostics,
        }
    }

    /// Enter a read query section for this workspace.
    pub(super) fn enter_query(&self) -> RwLockReadGuard<'_, ()> {
        self.compile_lock.read()
    }

    /// Enter a mutation section for this workspace.
    pub(super) fn enter_mutation(&self) -> RwLockWriteGuard<'_, ()> {
        self.compile_lock.write()
    }

    /// Track one open document by path.
    pub(super) fn set_tracked_document(&self, path: &Path, uri: Uri, version: i32) {
        let key = tracked_document_key(path);
        let state = TrackedDocumentState {
            path: path.to_path_buf(),
            uri,
            version,
        };
        let current = self.tracked_documents_by_path.read().clone();
        let mut tracked_documents = (*current).clone();
        tracked_documents.insert(key, state);
        *self.tracked_documents_by_path.write() = Arc::new(tracked_documents);
    }

    /// Set one overlay projection when available.
    pub(super) fn set_overlay_for_path(&self, path: &Path, text: String) {
        let Some(overlay_fs) = self.overlay_fs.as_ref() else {
            return;
        };

        overlay_fs.set_overlay(path, text);
    }

    /// Remove one overlay projection when available.
    pub(super) fn remove_overlay_for_path(&self, path: &Path) {
        let Some(overlay_fs) = self.overlay_fs.as_ref() else {
            return;
        };

        overlay_fs.remove_overlay(path);
    }

    /// Return true when a path is tracked as one open document.
    pub(super) fn has_tracked_document_for_path(&self, path: &Path) -> bool {
        let key = tracked_document_key(path);
        self.tracked_documents_by_path.read().contains_key(&key)
    }

    /// Return one tracked document identity for a path.
    pub(super) fn tracked_document_identity_for_path(&self, path: &Path) -> Option<(Uri, i32)> {
        let key = tracked_document_key(path);
        let current = self.tracked_documents_by_path.read().clone();
        let entry = current.get(&key)?;

        Some((entry.uri.clone(), entry.version))
    }

    /// Return one tracked document snapshot for a path.
    pub(super) fn tracked_document_for_path(&self, path: &Path) -> Option<(Uri, i32, String)> {
        let (uri, version) = self.tracked_document_identity_for_path(path)?;
        let overlay_fs = self.overlay_fs.as_ref()?;
        let text = overlay_fs.read_to_string(path).ok()?;

        Some((uri, version, text))
    }

    /// Return the tracked open documents keyed by path.
    pub(super) fn tracked_document_identities(&self) -> Vec<(PathBuf, Uri, i32)> {
        self.tracked_documents_by_path
            .read()
            .iter()
            .map(|entry| {
                let (_path, state) = entry;
                (state.path.clone(), state.uri.clone(), state.version)
            })
            .collect()
    }

    /// Stop tracking one open document and return its last known state.
    pub(super) fn untrack_document(&self, path: &Path) -> Option<(Uri, i32)> {
        let key = tracked_document_key(path);
        let current = self.tracked_documents_by_path.read().clone();
        let state = current.get(&key)?.clone();
        let mut tracked_documents = (*current).clone();
        tracked_documents.remove(&key);
        *self.tracked_documents_by_path.write() = Arc::new(tracked_documents);

        Some((state.uri, state.version))
    }
}

/// Internal update with impact metadata.
#[derive(Debug, Clone)]
pub(super) struct ServiceUpdate {
    /// Updated module id when known.
    pub(super) module_id: Option<ModuleId>,
    /// Updated file id.
    pub(super) file_id: FileId,
    /// Updated file snapshot.
    pub(super) file: FileSnapshot,
    /// Impact summary.
    pub(super) impact: UpdateImpact,
    /// Diagnostics for the updated file.
    pub(super) diagnostics: Vec<Diagnostic>,
}

/// Build an internal update from repository state.
pub(super) fn build_update(
    repository: &Repository,
    revision: Revision,
    module_id: Option<ModuleId>,
    file_id: FileId,
    impact: UpdateImpact,
) -> Result<ServiceUpdate, LanguageServiceError> {
    // prefer the explicit module id, otherwise recover it from the post update impact
    let module_id = module_id.or_else(|| impact.modules.first().copied());
    let file = file_snapshot_for_id(repository, revision, file_id)?;

    Ok(ServiceUpdate {
        module_id,
        file_id,
        file,
        impact,
        diagnostics: Vec::new(),
    })
}

/// Build a file snapshot for a repository file id.
pub(super) fn file_snapshot_for_id(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Result<FileSnapshot, LanguageServiceError> {
    let file = repository
        .file(revision, file_id)
        .map_err(LanguageServiceError::from)?
        .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;

    Ok(file_snapshot_from_file(&file))
}

/// Build a file snapshot payload.
pub(super) fn file_snapshot_from_file(file: &File) -> FileSnapshot {
    let content = match &file.content {
        FileContent::Text { content } => Some(content.clone()),
        FileContent::Json { content, .. } => Some(content.clone()),
        FileContent::Binary { .. } => None,
        FileContent::Missing => None,
        FileContent::Unloaded => None,
    };

    FileSnapshot {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    }
}

/// Build a workspace message.
pub(super) fn workspace_message(
    kind: WorkspaceMessageKind,
    code: &str,
    message: &str,
) -> WorkspaceMessage {
    WorkspaceMessage {
        kind,
        code: code.to_string(),
        message: message.to_string(),
    }
}

/// Build a warning message.
pub(super) fn warning_message(code: &str, message: &str) -> WorkspaceMessage {
    workspace_message(WorkspaceMessageKind::Warning, code, message)
}
