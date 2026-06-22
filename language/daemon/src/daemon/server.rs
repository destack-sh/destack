use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use destack_repository::Repository;
use destack_session::{Session, SessionEventHandler};
use destack_workspace::{
    Server, ServerActivity, ServerLifecycle, ServerOptions, WorkspaceEndpoint,
    WorkspaceEndpointError, WorkspaceIpcError, WorkspaceIpcListener, WorkspaceServerMetadata,
    WorkspaceWebSocketError, WorkspaceWebSocketListener,
};

use super::constants::{DEFAULT_IDLE_SHUTDOWN_MS, IDLE_SHUTDOWN_POLL_MS};
use crate::{Daemon, DaemonError};

/// Options for the workspace server.
#[derive(Clone)]
pub struct WorkspaceServerOptions {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for daemon progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Protocol options for workspace connections.
    pub protocol: ServerOptions,
    /// Idle shutdown timeout.
    pub idle_shutdown: Option<Duration>,
}

impl std::fmt::Debug for WorkspaceServerOptions {
    /// Format the visible workspace server options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceServerOptions")
            .field("worker_limit", &self.worker_limit)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field("protocol", &self.protocol)
            .field("idle_shutdown", &self.idle_shutdown)
            .finish()
    }
}

/// Running process bound to workspace connection transports.
#[derive(Debug)]
pub struct WorkspaceServer {
    /// Workspace endpoint metadata.
    endpoint: WorkspaceEndpoint,
    /// The daemon backing this server.
    daemon: Arc<Daemon>,
    /// Server options in use.
    options: WorkspaceServerOptions,
    /// Shutdown flag shared across connections.
    shutdown: Arc<AtomicBool>,
}

impl WorkspaceServer {
    /// Create a new workspace server for a repository and endpoint.
    pub fn new(
        repository: Arc<Repository>,
        endpoint: WorkspaceEndpoint,
    ) -> Result<Self, WorkspaceServerError> {
        // use server defaults
        let options = WorkspaceServerOptions::default();

        // build the server state
        Self::with_options(repository, endpoint, options)
    }

    /// Create a workspace server with explicit options.
    pub fn with_options(
        repository: Arc<Repository>,
        endpoint: WorkspaceEndpoint,
        options: WorkspaceServerOptions,
    ) -> Result<Self, WorkspaceServerError> {
        // build the daemon state
        let daemon = Arc::new(Daemon::new(
            repository,
            options.worker_limit,
            options.session_event_handler.clone(),
        )?);

        // return the server state
        Ok(Self {
            endpoint,
            daemon,
            options,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Serve workspace requests until shutdown.
    pub fn serve(&self) -> Result<(), WorkspaceServerError> {
        // lock the workspace endpoint
        let _lock = self
            .endpoint
            .lock()
            .map_err(WorkspaceServerError::Endpoint)?;

        // clear any stale socket path
        self.endpoint
            .clear_socket_path()
            .map_err(WorkspaceServerError::Endpoint)?;

        // create browser connection token
        let websocket_token = self
            .endpoint
            .create_websocket_token()
            .map_err(WorkspaceServerError::Endpoint)?;

        // bind the ipc and websocket listeners
        let max_frame_bytes = self.options.protocol.limits.max_frame_bytes as usize;
        let ipc_listener = WorkspaceIpcListener::bind(&self.endpoint.socket_path, max_frame_bytes)
            .map_err(WorkspaceServerError::Transport)?;
        let websocket_listener = WorkspaceWebSocketListener::bind(
            self.endpoint.websocket_addr,
            max_frame_bytes,
            self.endpoint.websocket_path.clone(),
            websocket_token.clone(),
        )
        .map_err(WorkspaceServerError::WebSocket)?;
        let websocket_addr = websocket_listener
            .local_addr()
            .map_err(WorkspaceServerError::WebSocket)?;

        // write workspace server metadata
        let metadata =
            WorkspaceServerMetadata::new(&self.endpoint, websocket_addr, &websocket_token)
                .map_err(WorkspaceServerError::Endpoint)?;
        self.endpoint
            .write_metadata(&metadata)
            .map_err(WorkspaceServerError::Endpoint)?;

        // serve incoming connections
        let result = self.serve_listeners(ipc_listener, websocket_listener);

        // clean up workspace metadata and socket path
        let _ = self.endpoint.remove_metadata();
        let _ = self.endpoint.clear_socket_path();

        // return the serve result
        result
    }

    /// Serve workspace connections from listeners.
    fn serve_listeners(
        &self,
        ipc_listener: WorkspaceIpcListener,
        websocket_listener: WorkspaceWebSocketListener,
    ) -> Result<(), WorkspaceServerError> {
        // initialize connection state
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let activity = Arc::new(ServerActivity::new(self.options.idle_shutdown));
        let lifecycle = ServerLifecycle::with_activity(self.shutdown.clone(), activity);
        let monitor = self.spawn_idle_monitor(lifecycle.clone());

        // accept connections until shutdown
        loop {
            // exit when shutdown is requested
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            // accept the next ready transports
            let ipc_transport = ipc_listener
                .try_accept()
                .map_err(WorkspaceServerError::Transport)?;
            let websocket_transport = websocket_listener
                .try_accept()
                .map_err(WorkspaceServerError::WebSocket)?;

            // spawn connection handlers for accepted transports
            let mut accepted_any = false;
            let accepted = ipc_transport.into_iter().chain(websocket_transport);
            for transport in accepted {
                accepted_any = true;
                let daemon = self.daemon.clone();
                let options = self.options.protocol.clone();
                let lifecycle = lifecycle.clone();
                let handle = std::thread::spawn(move || {
                    let server = Server::with_lifecycle(daemon, options, lifecycle);
                    let _ = server.serve(transport.as_ref());
                });
                handles.push(handle);
            }

            // wait briefly when neither listener produced work
            if !accepted_any {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }

        // join the idle monitor if it was spawned
        if let Some(handle) = monitor {
            let _ = handle.join();
        }

        // join all connection threads
        for handle in handles {
            let _ = handle.join();
        }

        // signal success
        Ok(())
    }

    /// Spawn an idle shutdown monitor when configured.
    fn spawn_idle_monitor(&self, lifecycle: ServerLifecycle) -> Option<JoinHandle<()>> {
        // return early when idle shutdown is disabled
        self.options.idle_shutdown?;

        // capture shared shutdown state
        let shutdown = self.shutdown.clone();
        let poll_interval = Duration::from_millis(IDLE_SHUTDOWN_POLL_MS);

        // spawn the idle monitor thread
        Some(std::thread::spawn(move || {
            loop {
                // exit if shutdown already requested
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                // shut down when the workspace server is idle
                if lifecycle.should_shutdown() {
                    lifecycle.request_shutdown();
                    break;
                }

                // wait for the next poll interval
                std::thread::sleep(poll_interval);
            }
        }))
    }
}

/// Errors returned by workspace servers.
#[derive(Debug)]
pub enum WorkspaceServerError {
    /// Endpoint error.
    Endpoint(WorkspaceEndpointError),
    /// Transport error.
    Transport(WorkspaceIpcError),
    /// WebSocket transport error.
    WebSocket(WorkspaceWebSocketError),
    /// Daemon state error.
    Daemon(DaemonError),
}

impl std::fmt::Display for WorkspaceServerError {
    /// Format the workspace server error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceServerError::Endpoint(error) => {
                write!(f, "workspace server endpoint error: {error}")
            }
            WorkspaceServerError::Transport(error) => {
                write!(f, "workspace server transport error: {error}")
            }
            WorkspaceServerError::WebSocket(error) => {
                write!(f, "workspace server websocket error: {error}")
            }
            WorkspaceServerError::Daemon(error) => {
                write!(f, "workspace server state error: {error}")
            }
        }
    }
}

impl std::error::Error for WorkspaceServerError {}

impl From<WorkspaceEndpointError> for WorkspaceServerError {
    /// Convert an endpoint error into a server error.
    fn from(error: WorkspaceEndpointError) -> Self {
        WorkspaceServerError::Endpoint(error)
    }
}

impl From<WorkspaceIpcError> for WorkspaceServerError {
    /// Convert an ipc transport error into a server error.
    fn from(error: WorkspaceIpcError) -> Self {
        WorkspaceServerError::Transport(error)
    }
}

impl From<WorkspaceWebSocketError> for WorkspaceServerError {
    /// Convert a WebSocket error into a server error.
    fn from(error: WorkspaceWebSocketError) -> Self {
        WorkspaceServerError::WebSocket(error)
    }
}

impl From<DaemonError> for WorkspaceServerError {
    /// Convert a daemon error into a server error.
    fn from(error: DaemonError) -> Self {
        WorkspaceServerError::Daemon(error)
    }
}

impl Default for WorkspaceServerOptions {
    /// Create default workspace server options.
    fn default() -> Self {
        Self {
            worker_limit: Session::default_worker_count(),
            session_event_handler: None,
            protocol: ServerOptions::default(),
            idle_shutdown: Some(Duration::from_millis(DEFAULT_IDLE_SHUTDOWN_MS)),
        }
    }
}
