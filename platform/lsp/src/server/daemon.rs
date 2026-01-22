use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use destack_daemon::protocol::{
    AnalyzeRequest, CloseWorkspaceRequest, DaemonMessageRecord, DaemonRequest, DaemonResponse,
    DaemonUpdateRecord, FileUpdate, FileUpdateKind, FileUpdateRequest, OpenWorkspaceRequest,
    ProtocolClient, ProtocolClientOptions, RescanReason, RescanWorkspaceRequest, WatchBatch,
    WatchBatchRequest, WatchEvent, WorkspaceHandleId, WorkspaceOpenOptions,
    loopback_transport_pair,
};
use destack_daemon::{DaemonService, DaemonServiceError, DaemonServiceOptions};
use destack_source::{FileWatchEvent, FileWatchEventKind};
use destack_workspace::Session;
use parking_lot::Mutex;

/// Result of applying daemon updates in the LSP.
#[derive(Debug, Default)]
pub struct LspDaemonResult {
    /// Update records produced by the daemon.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Message records produced by the daemon.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Protocol backed daemon client for LSP updates.
#[derive(Debug)]
pub struct LspDaemonClient {
    /// Session for workspace resolution.
    session: Arc<Session>,
    /// Protocol client used for requests.
    client: ProtocolClient,
    /// Workspace handle ids keyed by root.
    handles: DashMap<PathBuf, WorkspaceHandleId>,
    /// Server thread handle.
    server_handle: Mutex<Option<JoinHandle<Result<(), DaemonServiceError>>>>,
}

impl LspDaemonClient {
    /// Create a protocol daemon client for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, String> {
        // configure the daemon service
        let options = DaemonServiceOptions::default();
        let service = DaemonService::with_options(session.clone(), options);

        // start the protocol server over a loopback transport
        let (client_transport, server_transport) = loopback_transport_pair(16);
        let server_handle = thread::spawn(move || service.serve_transport(&server_transport));

        // build the protocol client and handshake
        let client = ProtocolClient::new(Arc::new(client_transport));
        if let Err(error) = client.handshake(ProtocolClientOptions::default()) {
            shutdown_server(&client, server_handle);
            return Err(format!("daemon handshake failed: {error}"));
        }

        let daemon = Self {
            session,
            client,
            handles: DashMap::new(),
            server_handle: Mutex::new(Some(server_handle)),
        };

        // open each workspace root
        for root in roots {
            daemon.open_workspace_root(root)?;
        }

        Ok(daemon)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), String> {
        if self.handles.contains_key(&root) {
            return Ok(());
        }

        let options = WorkspaceOpenOptions {
            watch: false,
            ..Default::default()
        };
        let request = OpenWorkspaceRequest {
            root: root.clone(),
            options,
        };
        let response = self
            .client
            .send_request(DaemonRequest::OpenWorkspace(request))
            .map_err(|error| format!("open workspace failed: {error}"))?;

        let handle = match response {
            DaemonResponse::WorkspaceOpened(response) => response.handle,
            DaemonResponse::Error(error) => {
                return Err(format!("open workspace failed: {error}"));
            }
            other => {
                return Err(format!("unexpected response: {other:?}"));
            }
        };

        self.handles.insert(root, handle);
        Ok(())
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), String> {
        let Some(entry) = self.handles.remove(root) else {
            return Ok(());
        };

        let handle = entry.1;
        let response = self
            .client
            .send_request(DaemonRequest::CloseWorkspace(CloseWorkspaceRequest {
                handle,
            }))
            .map_err(|error| format!("close workspace failed: {error}"))?;

        match response {
            DaemonResponse::WorkspaceClosed(_) => Ok(()),
            DaemonResponse::Error(error) => Err(format!("close workspace failed: {error}")),
            other => Err(format!("unexpected response: {other:?}")),
        }
    }

    /// Ensure a path is analyzed by the daemon.
    pub fn ensure_analyzed_for_path(&self, path: &Path) -> Result<(), String> {
        let handle = self.handle_for_path(path)?;
        let request = AnalyzeRequest {
            handle,
            path: path.to_path_buf(),
        };
        let response = self
            .client
            .send_request(DaemonRequest::Analyze(request))
            .map_err(|error| format!("analyze request failed: {error}"))?;

        match response {
            DaemonResponse::Analyzed(_) => Ok(()),
            DaemonResponse::Error(error) => Err(format!("analyze request failed: {error}")),
            other => Err(format!("unexpected response: {other:?}")),
        }
    }

    /// Apply a virtual file update through the daemon.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<LspDaemonResult, String> {
        let handle = self.handle_for_path(path)?;
        let update = FileUpdate {
            path: path.to_path_buf(),
            update: FileUpdateKind::Text { content },
            write_to_disk: false,
        };
        let response = self
            .client
            .send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
                handle,
                update,
            }))
            .map_err(|error| format!("apply file update failed: {error}"))?;

        match response {
            DaemonResponse::FileUpdated(response) => Ok(LspDaemonResult {
                updates: response.updates,
                messages: response.messages,
            }),
            DaemonResponse::Error(error) => Err(format!("apply file update failed: {error}")),
            other => Err(format!("unexpected response: {other:?}")),
        }
    }

    /// Apply watch events through the daemon and handle rescans.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<LspDaemonResult, String> {
        let mut result = LspDaemonResult::default();
        if events.is_empty() {
            return Ok(result);
        }

        for entry in self.handles.iter() {
            let root = entry.key();
            let handle = *entry.value();
            let batch = watch_batch_for_root(root, &events);
            let Some(batch) = batch else {
                continue;
            };

            let response = self
                .client
                .send_request(DaemonRequest::ApplyWatchBatch(WatchBatchRequest {
                    handle,
                    batch,
                }))
                .map_err(|error| format!("apply watch batch failed: {error}"))?;

            let response = match response {
                DaemonResponse::WatchBatchApplied(response) => response,
                DaemonResponse::Error(error) => {
                    return Err(format!("apply watch batch failed: {error}"));
                }
                other => {
                    return Err(format!("unexpected response: {other:?}"));
                }
            };

            result.updates.extend(response.updates);
            result.messages.extend(response.messages);

            if response.rescan {
                let rescan = self.rescan_handle(handle, RescanReason::Update)?;
                result.updates.extend(rescan.updates);
                result.messages.extend(rescan.messages);
            }
        }

        Ok(result)
    }

    /// Shutdown the daemon connection.
    pub fn shutdown(&self) {
        let _ = self.client.send_request(DaemonRequest::Shutdown);
        let handle = self.server_handle.lock().take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }

    fn handle_for_root(&self, root: &Path) -> Option<WorkspaceHandleId> {
        self.handles.get(root).map(|entry| *entry.value())
    }

    fn handle_for_path(&self, path: &Path) -> Result<WorkspaceHandleId, String> {
        let program = self.session.find_program_for_path(path);
        let root = program.cwd.clone();
        if let Some(handle) = self.handle_for_root(&root) {
            return Ok(handle);
        }

        self.open_workspace_root(root.clone())?;
        self.handle_for_root(&root)
            .ok_or_else(|| "workspace handle missing after open".to_string())
    }

    fn rescan_handle(
        &self,
        handle: WorkspaceHandleId,
        reason: RescanReason,
    ) -> Result<LspDaemonResult, String> {
        let response = self
            .client
            .send_request(DaemonRequest::RescanWorkspace(RescanWorkspaceRequest {
                handle,
                reason,
            }))
            .map_err(|error| format!("rescan workspace failed: {error}"))?;

        match response {
            DaemonResponse::WorkspaceRescanned(response) => Ok(LspDaemonResult {
                updates: response.updates,
                messages: response.messages,
            }),
            DaemonResponse::Error(error) => Err(format!("rescan workspace failed: {error}")),
            other => Err(format!("unexpected response: {other:?}")),
        }
    }
}

impl Drop for LspDaemonClient {
    fn drop(&mut self) {
        // shut down the server thread on drop
        if let Some(handle) = self.server_handle.lock().take() {
            let _ = self.client.send_request(DaemonRequest::Shutdown);
            let _ = handle.join();
        }
    }
}

fn watch_batch_for_root(root: &Path, events: &[FileWatchEvent]) -> Option<WatchBatch> {
    let filtered: Vec<WatchEvent> = events
        .iter()
        .filter(|event| event_matches_root(event, root))
        .map(WatchEvent::from)
        .collect();
    if filtered.is_empty() {
        return None;
    }

    let overflowed = events
        .iter()
        .filter(|event| event_matches_root(event, root))
        .any(|event| matches!(event.kind, FileWatchEventKind::Overflow));
    let (started_at_ns, ended_at_ns) = watch_batch_timestamps();
    Some(WatchBatch {
        events: filtered,
        status: Vec::new(),
        overflowed,
        started_at_ns,
        ended_at_ns,
    })
}

fn event_matches_root(event: &FileWatchEvent, root: &Path) -> bool {
    if event.path.starts_with(root) {
        return true;
    }

    event
        .previous_path
        .as_ref()
        .is_some_and(|path| path.starts_with(root))
}

fn watch_batch_timestamps() -> (u64, u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration_to_ns(now), duration_to_ns(now))
}

fn duration_to_ns(duration: Duration) -> u64 {
    let nanos = duration.as_nanos();
    if nanos > u64::MAX as u128 {
        u64::MAX
    } else {
        nanos as u64
    }
}

fn shutdown_server(
    client: &ProtocolClient,
    server_handle: JoinHandle<Result<(), DaemonServiceError>>,
) {
    let _ = client.send_request(DaemonRequest::Shutdown);
    let _ = server_handle.join();
}
