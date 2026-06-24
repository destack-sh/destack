use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use destack_repository::Repository;
use destack_session::{Session, SessionEventHandler};
use destack_workspace::{
    IpcError, IpcListener, Metadata, Server, ServerActivity, ServerLifecycle, ServerOptions,
    Service, ServiceError, WebSocketError, WebSocketListener,
};

use super::constants::{DEFAULT_IDLE_SHUTDOWN_MS, IDLE_SHUTDOWN_POLL_MS};
use crate::{Daemon, DaemonError, OpenedWorkspace};

/// Options for the daemon server.
#[derive(Clone)]
pub struct DaemonServerOptions {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for daemon progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Protocol options for workspace connections.
    pub protocol: ServerOptions,
    /// Idle shutdown timeout.
    pub idle_shutdown: Option<Duration>,
}

impl std::fmt::Debug for DaemonServerOptions {
    /// Format the visible daemon server options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonServerOptions")
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
pub struct DaemonServer {
    /// Workspace service metadata.
    service: Service,
    /// Workspace exposed by this daemon service.
    workspace: Arc<OpenedWorkspace>,
    /// Daemon server options in use.
    options: DaemonServerOptions,
    /// Shutdown flag shared across connections.
    shutdown: Arc<AtomicBool>,
}

impl DaemonServer {
    /// Create a new daemon server for a repository and service.
    pub fn new(repository: Arc<Repository>, service: Service) -> Result<Self, DaemonServerError> {
        // use server defaults
        let options = DaemonServerOptions::default();

        // build the server state
        Self::with_options(repository, service, options)
    }

    /// Create a daemon server with explicit options.
    pub fn with_options(
        repository: Arc<Repository>,
        service: Service,
        options: DaemonServerOptions,
    ) -> Result<Self, DaemonServerError> {
        // build the daemon state
        let workspace_root = repository.path().to_path_buf();
        let daemon = Arc::new(Daemon::new(
            repository,
            options.worker_limit,
            options.session_event_handler.clone(),
        )?);
        let workspace = daemon.open(&workspace_root)?;

        // return the server state
        Ok(Self {
            service,
            workspace,
            options,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Serve workspace requests until shutdown.
    pub fn serve(&self) -> Result<(), DaemonServerError> {
        // lock the workspace service
        let _lock = self.service.lock().map_err(DaemonServerError::Service)?;

        // clear any stale socket path
        self.service
            .clear_socket_path()
            .map_err(DaemonServerError::Service)?;

        // create browser connection token
        let websocket_token = self
            .service
            .create_websocket_token()
            .map_err(DaemonServerError::Service)?;

        // bind the ipc and websocket listeners
        let max_frame_bytes = self.options.protocol.limits.max_frame_bytes as usize;
        let ipc_listener = IpcListener::bind(&self.service.socket_path, max_frame_bytes)
            .map_err(DaemonServerError::Transport)?;
        let websocket_listener = WebSocketListener::bind(
            self.service.websocket_addr,
            max_frame_bytes,
            self.service.websocket_path.clone(),
            websocket_token.clone(),
        )
        .map_err(DaemonServerError::WebSocket)?;
        let websocket_addr = websocket_listener
            .local_addr()
            .map_err(DaemonServerError::WebSocket)?;

        // write daemon server metadata
        let metadata = Metadata::new(&self.service, websocket_addr, &websocket_token)
            .map_err(DaemonServerError::Service)?;
        self.service
            .write_metadata(&metadata)
            .map_err(DaemonServerError::Service)?;

        // serve incoming connections
        let result = self.serve_listeners(ipc_listener, websocket_listener);

        // clean up workspace metadata and socket path
        let _ = self.service.remove_metadata();
        let _ = self.service.clear_socket_path();

        // return the serve result
        result
    }

    /// Serve workspace connections from listeners.
    fn serve_listeners(
        &self,
        ipc_listener: IpcListener,
        websocket_listener: WebSocketListener,
    ) -> Result<(), DaemonServerError> {
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
                .map_err(DaemonServerError::Transport)?;
            let websocket_transport = websocket_listener
                .try_accept()
                .map_err(DaemonServerError::WebSocket)?;

            // spawn connection handlers for accepted transports
            let mut accepted_any = false;
            let accepted = ipc_transport.into_iter().chain(websocket_transport);
            for transport in accepted {
                accepted_any = true;
                let workspace = self.workspace.workspace();
                let root_lease = self.workspace.root_lease();
                let options = self.options.protocol.clone();
                let lifecycle = lifecycle.clone();
                let handle = std::thread::spawn(move || {
                    let server = Server::with_root_lease(workspace, root_lease, options, lifecycle);
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

                // shut down when the daemon server is idle
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

/// Errors returned by daemon servers.
#[derive(Debug)]
pub enum DaemonServerError {
    /// Workspace service error.
    Service(ServiceError),
    /// Transport error.
    Transport(IpcError),
    /// WebSocket transport error.
    WebSocket(WebSocketError),
    /// Daemon state error.
    Daemon(DaemonError),
}

impl std::fmt::Display for DaemonServerError {
    /// Format the daemon server error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonServerError::Service(error) => {
                write!(f, "daemon server service error: {error}")
            }
            DaemonServerError::Transport(error) => {
                write!(f, "daemon server transport error: {error}")
            }
            DaemonServerError::WebSocket(error) => {
                write!(f, "daemon server websocket error: {error}")
            }
            DaemonServerError::Daemon(error) => {
                write!(f, "daemon server state error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonServerError {}

impl From<ServiceError> for DaemonServerError {
    /// Convert a service error into a server error.
    fn from(error: ServiceError) -> Self {
        DaemonServerError::Service(error)
    }
}

impl From<IpcError> for DaemonServerError {
    /// Convert an ipc transport error into a server error.
    fn from(error: IpcError) -> Self {
        DaemonServerError::Transport(error)
    }
}

impl From<WebSocketError> for DaemonServerError {
    /// Convert a WebSocket error into a server error.
    fn from(error: WebSocketError) -> Self {
        DaemonServerError::WebSocket(error)
    }
}

impl From<DaemonError> for DaemonServerError {
    /// Convert a daemon error into a server error.
    fn from(error: DaemonError) -> Self {
        DaemonServerError::Daemon(error)
    }
}

impl Default for DaemonServerOptions {
    /// Create default daemon server options.
    fn default() -> Self {
        Self {
            worker_limit: Session::default_worker_count(),
            session_event_handler: None,
            protocol: ServerOptions::default(),
            idle_shutdown: Some(Duration::from_millis(DEFAULT_IDLE_SHUTDOWN_MS)),
        }
    }
}
