use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_daemon::protocol::{
    DiagnosticSnapshot, FileImagesRequest, FileSnapshot, FileSnapshotRequest, FileUpdate,
    FileUpdateKind, ProtocolClient, QueryRequestBody, QueryResponseBody, RootHandleId,
    RootOpenOptions, RootSnapshot,
};
use destack_daemon::{
    DaemonConnectError, DaemonConnectOptions, DaemonEndpoint, DaemonLaunch, connect_ipc_daemon,
};
use destack_source::{TextChange, TextPosition, TextRange, apply_text_changes};
use destack_workspace::{Repository, Revision};

/// Editor-side daemon workspace used by the LSP adapter.
#[derive(Debug)]
pub(super) struct DaemonWorkspace {
    /// Protocol client connected to the language daemon.
    client: Arc<ProtocolClient>,
    /// Workspace root used for daemon root routing.
    workspace_root: PathBuf,
    /// Open root handles keyed by root path.
    root_by_path: DashMap<PathBuf, RootHandleId>,
    /// Open editor text keyed by source path.
    text_by_path: DashMap<PathBuf, OpenText>,
}

impl DaemonWorkspace {
    /// Connect to the language daemon and open the requested roots.
    pub(super) fn connect(
        repository: &Repository,
        roots: Vec<PathBuf>,
    ) -> Result<Self, DaemonWorkspaceError> {
        let endpoint = DaemonEndpoint::new(repository.layout().home.clone());
        let root = repository.workspace_root().to_path_buf();
        let launch = DaemonLaunch::for_endpoint(&endpoint, root);
        let connection =
            connect_ipc_daemon(&endpoint, DaemonConnectOptions::default(), Some(launch))
                .map_err(DaemonWorkspaceError::Connect)?;
        let workspace = Self {
            client: connection.client,
            workspace_root: repository.workspace_root().to_path_buf(),
            root_by_path: DashMap::new(),
            text_by_path: DashMap::new(),
        };

        for root in roots {
            workspace.open_root(root)?;
        }

        Ok(workspace)
    }

    /// Open one root and retain its daemon handle.
    pub(super) fn open_root(&self, root: PathBuf) -> Result<(), DaemonWorkspaceError> {
        let response = self
            .client
            .open_root(
                self.workspace_root.clone(),
                root.clone(),
                RootOpenOptions::default(),
            )
            .map_err(DaemonWorkspaceError::Protocol)?;

        self.root_by_path.insert(root, response.handle);

        Ok(())
    }

    /// Close one root handle.
    pub(super) fn close_root(&self, root: &Path) -> Result<(), DaemonWorkspaceError> {
        let Some((_, handle)) = self.root_by_path.remove(root) else {
            return Ok(());
        };

        self.client
            .close_root(handle)
            .map_err(DaemonWorkspaceError::Protocol)?;

        Ok(())
    }

    /// Return the open root that owns one path.
    pub(super) fn root_for_path(&self, path: &Path) -> Result<PathBuf, DaemonWorkspaceError> {
        let mut best: Option<(usize, PathBuf)> = None;
        for entry in self.root_by_path.iter() {
            let root = entry.key();
            if !path.starts_with(root) {
                continue;
            }
            let size = root.as_os_str().len();
            if best.as_ref().is_none_or(|(best_size, _)| size > *best_size) {
                best = Some((size, root.clone()));
            }
        }

        best.map(|(_, root)| root)
            .ok_or_else(|| DaemonWorkspaceError::PathNotInRoot {
                path: path.to_path_buf(),
            })
    }

    /// Return true when a path is currently open in the editor.
    pub(super) fn has_open_file(&self, path: &Path) -> bool {
        self.text_by_path.contains_key(path)
    }

    /// Open one editor file through the daemon.
    pub(super) fn open_file(
        &self,
        path: PathBuf,
        version: i32,
        text: String,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        self.text_by_path.insert(
            path.clone(),
            OpenText {
                version,
                text: text.clone(),
            },
        );

        self.replace_text(path, text)
    }

    /// Apply incremental editor changes and replace daemon text.
    pub(super) fn change_file(
        &self,
        path: PathBuf,
        version: i32,
        changes: Vec<destack_lsp_types::TextDocumentContentChangeEvent>,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let mut entry = self
            .text_by_path
            .get_mut(&path)
            .ok_or_else(|| DaemonWorkspaceError::FileNotOpen { path: path.clone() })?;
        let text_changes = text_changes_from_lsp(changes);
        let text = apply_text_changes(entry.text.clone(), &text_changes)
            .map_err(|error| DaemonWorkspaceError::TextChange(error.to_string()))?;
        entry.version = version;
        entry.text = text.clone();
        drop(entry);

        self.replace_text(path, text)
    }

    /// Save one editor file through the daemon.
    pub(super) fn save_file(
        &self,
        path: PathBuf,
        text: Option<String>,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let text = if let Some(text) = text {
            text
        } else {
            self.text_by_path
                .get(&path)
                .map(|entry| entry.text.clone())
                .ok_or_else(|| DaemonWorkspaceError::FileNotOpen { path: path.clone() })?
        };

        self.replace_text(path, text)
    }

    /// Close one editor file through the daemon.
    pub(super) fn close_file(
        &self,
        path: PathBuf,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        // remove local overlay state optimistically
        let open_text = self.text_by_path.remove(&path);
        let handle = match self.handle_for_path(&path) {
            Ok(handle) => handle,
            Err(error) => {
                if let Some((path, text)) = open_text {
                    self.text_by_path.insert(path, text);
                }

                return Err(error);
            }
        };

        // close daemon overlay state
        let result = self
            .client
            .apply_file_update(
                handle,
                FileUpdate {
                    path,
                    update: FileUpdateKind::Closed,
                    write_to_disk: false,
                },
            )
            .map_err(DaemonWorkspaceError::Protocol);
        if let Err(error) = result {
            // restore local state when close fails
            if let Some((path, text)) = open_text {
                self.text_by_path.insert(path, text);
            }

            return Err(error);
        }

        self.diagnostic_snapshots_for_handle(handle)
    }

    /// Return diagnostics for one file.
    pub(super) fn file_diagnostics(
        &self,
        path: PathBuf,
    ) -> Result<Option<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let handle = self.handle_for_path(&path)?;

        self.client
            .file_diagnostics(handle, path)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Return diagnostics for all open roots.
    pub(super) fn diagnostics(&self) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let mut diagnostics = Vec::new();
        for handle in self.root_by_path.iter().map(|entry| *entry.value()) {
            diagnostics.extend(self.diagnostic_snapshots_for_handle(handle)?);
        }

        Ok(diagnostics)
    }

    /// Reload all open roots.
    pub(super) fn reload_all(&self) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let mut diagnostics = Vec::new();
        for handle in self.root_by_path.iter().map(|entry| *entry.value()) {
            self.client
                .reload_root(handle, destack_daemon::protocol::ReloadReason::Manual)
                .map_err(DaemonWorkspaceError::Protocol)?;
            diagnostics.extend(self.diagnostic_snapshots_for_handle(handle)?);
        }

        Ok(diagnostics)
    }

    /// Return a root snapshot for the path root.
    pub(super) fn root_snapshot(
        &self,
        path: &Path,
        target: Option<String>,
    ) -> Result<RootSnapshot, DaemonWorkspaceError> {
        let handle = self.handle_for_path(path)?;

        self.client
            .root_snapshot(handle, target)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Return a file snapshot for one path.
    pub(super) fn file_snapshot(
        &self,
        path: PathBuf,
        target: Option<String>,
    ) -> Result<Option<FileSnapshot>, DaemonWorkspaceError> {
        let handle = self.handle_for_path(&path)?;
        let request = FileSnapshotRequest { path, target };

        self.client
            .file_snapshot(handle, request)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Return source file images for one revision.
    pub(super) fn file_images(
        &self,
        path: &Path,
        revision: Revision,
        file_ids: Vec<destack_source::FileId>,
    ) -> Result<Vec<destack_daemon::protocol::FileUpdateImage>, DaemonWorkspaceError> {
        let handle = self.handle_for_path(path)?;
        let request = FileImagesRequest { revision, file_ids };

        self.client
            .file_images(handle, request)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Execute one semantic query.
    pub(super) fn execute_query(
        &self,
        path: &Path,
        request: destack_query::QueryRequest,
        revision: Revision,
    ) -> Result<QueryResponseBody, DaemonWorkspaceError> {
        let handle = self.handle_for_path(path)?;
        let request = QueryRequestBody {
            expected_revision: Some(revision),
            request,
        };

        self.client
            .execute_query(handle, request)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Replace the daemon text for one source file.
    fn replace_text(
        &self,
        path: PathBuf,
        text: String,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        let handle = self.handle_for_path(&path)?;
        self.client
            .apply_file_update(
                handle,
                FileUpdate {
                    path,
                    update: FileUpdateKind::Text { content: text },
                    write_to_disk: false,
                },
            )
            .map_err(DaemonWorkspaceError::Protocol)?;

        self.diagnostic_snapshots_for_handle(handle)
    }

    /// Return rich diagnostics for one root handle.
    fn diagnostic_snapshots_for_handle(
        &self,
        handle: RootHandleId,
    ) -> Result<Vec<DiagnosticSnapshot>, DaemonWorkspaceError> {
        self.client
            .diagnostic_snapshots(handle)
            .map_err(DaemonWorkspaceError::Protocol)
    }

    /// Find the daemon root handle for one path.
    fn handle_for_path(&self, path: &Path) -> Result<RootHandleId, DaemonWorkspaceError> {
        let mut best: Option<(usize, RootHandleId)> = None;
        for entry in self.root_by_path.iter() {
            let root = entry.key();
            if !path.starts_with(root) {
                continue;
            }
            let size = root.as_os_str().len();
            if best.is_none_or(|(best_size, _)| size > best_size) {
                best = Some((size, *entry.value()));
            }
        }

        best.map(|(_, handle)| handle)
            .ok_or_else(|| DaemonWorkspaceError::PathNotInRoot {
                path: path.to_path_buf(),
            })
    }
}

/// Open editor text cached by the LSP adapter.
#[derive(Debug, Clone)]
struct OpenText {
    /// Editor document version.
    version: i32,
    /// Current normalized text.
    text: String,
}

/// Errors produced by the daemon-backed LSP workspace.
#[derive(Debug)]
pub(super) enum DaemonWorkspaceError {
    /// Daemon connection failed.
    Connect(DaemonConnectError),
    /// Daemon protocol request failed.
    Protocol(destack_daemon::protocol::ProtocolClientError),
    /// Text change application failed.
    TextChange(String),
    /// Path is not inside an open root.
    PathNotInRoot { path: PathBuf },
    /// File was not opened before an incremental operation.
    FileNotOpen { path: PathBuf },
}

impl std::fmt::Display for DaemonWorkspaceError {
    /// Format the daemon workspace error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(error) => write!(formatter, "daemon connection failed: {error}"),
            Self::Protocol(error) => write!(formatter, "daemon protocol failed: {error}"),
            Self::TextChange(error) => write!(formatter, "text change failed: {error}"),
            Self::PathNotInRoot { path } => {
                write!(
                    formatter,
                    "path is outside daemon roots: {}",
                    path.display()
                )
            }
            Self::FileNotOpen { path } => {
                write!(formatter, "file is not open: {}", path.display())
            }
        }
    }
}

impl std::error::Error for DaemonWorkspaceError {}

/// Convert LSP text changes into source text changes.
fn text_changes_from_lsp(
    changes: Vec<destack_lsp_types::TextDocumentContentChangeEvent>,
) -> Vec<TextChange> {
    changes
        .into_iter()
        .map(|change| TextChange {
            range: change.range.map(|range| TextRange {
                start: TextPosition {
                    line: range.start.line,
                    character: range.start.character,
                },
                end: TextPosition {
                    line: range.end.line,
                    character: range.end.character,
                },
            }),
            text: change.text,
        })
        .collect()
}
