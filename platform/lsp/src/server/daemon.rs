use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use destack_daemon::protocol::{
    AnalyzeRequest, CloseWorkspaceRequest, DaemonMessageRecord, DaemonRequest, DaemonResponse,
    DaemonUpdateRecord, FileUpdate, FileUpdateKind, FileUpdateRequest, OpenWorkspaceRequest,
    ProtocolClient, RescanReason, RescanWorkspaceRequest, WatchBatch, WatchBatchRequest,
    WatchEvent, WorkspaceHandleId, WorkspaceOpenOptions,
};
use destack_daemon::{
    DaemonConnectOptions, DaemonConnection, DaemonInstance, DaemonLaunchConfig,
    connect_in_process_daemon, connect_ipc_daemon,
};
use destack_source::{FileWatchEvent, FileWatchEventKind};
use destack_workspace::Session;
use parking_lot::Mutex;

/// Environment variable that forces the LSP to use the in process daemon.
pub const LSP_DAEMON_IN_PROCESS_ENV: &str = "DESTACK_LSP_IN_PROCESS";

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
    client: Arc<ProtocolClient>,
    /// Workspace handle ids keyed by root.
    handles: DashMap<PathBuf, WorkspaceHandleId>,
    /// Connection state for in process daemons.
    connection: Mutex<Option<DaemonConnection>>,
}

impl LspDaemonClient {
    /// Create a protocol daemon client for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, String> {
        // force the in process daemon when requested
        if std::env::var_os(LSP_DAEMON_IN_PROCESS_ENV).is_some() {
            return Self::new_in_process(session, roots);
        }

        // build the ipc connection
        let instance = DaemonInstance::from_session(&session);
        let launch = build_launch_config(&session, &instance);
        let options = DaemonConnectOptions::default();
        let connection = connect_ipc_daemon(&instance, options, Some(launch))
            .map_err(|error| format!("daemon connect failed: {error}"))?;
        let client = connection.client.clone();

        // build the daemon client state
        let daemon = Self {
            session,
            client,
            handles: DashMap::new(),
            connection: Mutex::new(Some(connection)),
        };

        // open each workspace root
        for root in roots {
            daemon.open_workspace_root(root)?;
        }

        Ok(daemon)
    }

    /// Create an in process daemon client for the provided roots.
    pub fn new_in_process(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, String> {
        // build the in process connection
        let options = DaemonConnectOptions::default();
        let connection = connect_in_process_daemon(session.clone(), options)
            .map_err(|error| format!("daemon connect failed: {error}"))?;
        let client = connection.client.clone();

        // build the daemon client state
        let daemon = Self {
            session,
            client,
            handles: DashMap::new(),
            connection: Mutex::new(Some(connection)),
        };

        // open each workspace root
        for root in roots {
            daemon.open_workspace_root(root)?;
        }

        Ok(daemon)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), String> {
        // return early when the root is already open
        if self.handles.contains_key(&root) {
            return Ok(());
        }

        // build the open request
        let options = WorkspaceOpenOptions {
            watch: false,
            ..Default::default()
        };
        let request = OpenWorkspaceRequest {
            root: root.clone(),
            options,
        };

        // send the open request
        let response = self
            .client
            .send_request(DaemonRequest::OpenWorkspace(request))
            .map_err(|error| format!("open workspace failed: {error}"))?;

        // extract the workspace handle
        let handle = match response {
            DaemonResponse::WorkspaceOpened(response) => response.handle,
            DaemonResponse::Error(error) => {
                return Err(format!("open workspace failed: {error}"));
            }
            other => {
                return Err(format!("unexpected response: {other:?}"));
            }
        };

        // store the workspace handle
        self.handles.insert(root, handle);
        Ok(())
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), String> {
        // resolve the workspace handle
        let Some(entry) = self.handles.remove(root) else {
            return Ok(());
        };

        // send the close request
        let handle = entry.1;
        let response = self
            .client
            .send_request(DaemonRequest::CloseWorkspace(CloseWorkspaceRequest {
                handle,
            }))
            .map_err(|error| format!("close workspace failed: {error}"))?;

        // map the close response
        match response {
            DaemonResponse::WorkspaceClosed(_) => Ok(()),
            DaemonResponse::Error(error) => Err(format!("close workspace failed: {error}")),
            other => Err(format!("unexpected response: {other:?}")),
        }
    }

    /// Ensure a path is analyzed by the daemon.
    pub fn ensure_analyzed_for_path(&self, path: &Path) -> Result<(), String> {
        // resolve the workspace handle
        let handle = self.handle_for_path(path)?;

        // send the analyze request
        let request = AnalyzeRequest {
            handle,
            path: path.to_path_buf(),
        };
        let response = self
            .client
            .send_request(DaemonRequest::Analyze(request))
            .map_err(|error| format!("analyze request failed: {error}"))?;

        // map the analyze response
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
        // resolve the workspace handle
        let handle = self.handle_for_path(path)?;

        // build and send the update
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

        // map the update response
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
        // return empty results for empty inputs
        let mut result = LspDaemonResult::default();
        if events.is_empty() {
            return Ok(result);
        }

        // apply watch events per workspace
        for entry in self.handles.iter() {
            let root = entry.key();
            let handle = *entry.value();

            // build the watch batch for this root
            let batch = watch_batch_for_root(root, &events);
            let Some(batch) = batch else {
                continue;
            };

            // send the watch batch request
            let response = self
                .client
                .send_request(DaemonRequest::ApplyWatchBatch(WatchBatchRequest {
                    handle,
                    batch,
                }))
                .map_err(|error| format!("apply watch batch failed: {error}"))?;

            // map the watch batch response
            let response = match response {
                DaemonResponse::WatchBatchApplied(response) => response,
                DaemonResponse::Error(error) => {
                    return Err(format!("apply watch batch failed: {error}"));
                }
                other => {
                    return Err(format!("unexpected response: {other:?}"));
                }
            };

            // collect update records
            result.updates.extend(response.updates);
            result.messages.extend(response.messages);

            // rescan when requested by the daemon
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
        let _ = self.connection.lock().take();
    }

    /// Look up the workspace handle for a root.
    fn handle_for_root(&self, root: &Path) -> Option<WorkspaceHandleId> {
        self.handles.get(root).map(|entry| *entry.value())
    }

    /// Resolve the workspace handle for a path.
    fn handle_for_path(&self, path: &Path) -> Result<WorkspaceHandleId, String> {
        // resolve the root for the path
        let program = self.session.find_program_for_path(path);
        let root = program.cwd.clone();
        if let Some(handle) = self.handle_for_root(&root) {
            return Ok(handle);
        }

        // open the root on demand
        self.open_workspace_root(root.clone())?;
        self.handle_for_root(&root)
            .ok_or_else(|| "workspace handle missing after open".to_string())
    }

    /// Request a rescan for a workspace handle.
    fn rescan_handle(
        &self,
        handle: WorkspaceHandleId,
        reason: RescanReason,
    ) -> Result<LspDaemonResult, String> {
        // send the rescan request
        let response = self
            .client
            .send_request(DaemonRequest::RescanWorkspace(RescanWorkspaceRequest {
                handle,
                reason,
            }))
            .map_err(|error| format!("rescan workspace failed: {error}"))?;

        // map the rescan response
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

/// Build the launch config for the ipc daemon.
fn build_launch_config(session: &Session, instance: &DaemonInstance) -> DaemonLaunchConfig {
    let mut launch = DaemonLaunchConfig::for_instance(instance);
    launch.cwd = Some(session.cwd.clone());
    launch
}

/// Build a watch batch for a workspace root.
fn watch_batch_for_root(root: &Path, events: &[FileWatchEvent]) -> Option<WatchBatch> {
    // filter events that match the root
    let filtered: Vec<WatchEvent> = events
        .iter()
        .filter(|event| event_matches_root(event, root))
        .map(WatchEvent::from)
        .collect();
    if filtered.is_empty() {
        return None;
    }

    // detect overflow events for the root
    let overflowed = events
        .iter()
        .filter(|event| event_matches_root(event, root))
        .any(|event| matches!(event.kind, FileWatchEventKind::Overflow));

    // build the batch payload
    let (started_at_ns, ended_at_ns) = watch_batch_timestamps();
    Some(WatchBatch {
        events: filtered,
        status: Vec::new(),
        overflowed,
        started_at_ns,
        ended_at_ns,
    })
}

/// Return true when the event touches the workspace root.
fn event_matches_root(event: &FileWatchEvent, root: &Path) -> bool {
    // match current paths
    if event.path.starts_with(root) {
        return true;
    }

    // match previous paths
    event
        .previous_path
        .as_ref()
        .is_some_and(|path| path.starts_with(root))
}

/// Capture the start and end timestamps for a watch batch.
fn watch_batch_timestamps() -> (u64, u64) {
    // capture the current time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration_to_ns(now), duration_to_ns(now))
}

/// Convert a duration to a clamped nanosecond count.
fn duration_to_ns(duration: Duration) -> u64 {
    // return a clamped nanosecond count
    let nanos = duration.as_nanos();

    // clamp to u64 max on overflow
    if nanos > u64::MAX as u128 {
        return u64::MAX;
    }

    // use the exact nanosecond count
    nanos as u64
}
