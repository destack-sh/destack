use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use destack_repository::Repository;
use destack_session::{Session, SessionEventHandler};

use super::constants::{DEFAULT_IDLE_SHUTDOWN_MS, IDLE_SHUTDOWN_POLL_MS};
use super::{DaemonEndpoint, DaemonEndpointError, DaemonMetadata};
use crate::ipc::{DaemonIpcError, DaemonIpcListener};
use crate::{Daemon, DaemonError, protocol};

/// Options for the daemon server.
#[derive(Clone)]
pub struct DaemonServerOptions {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for daemon progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Protocol options for daemon connections.
    pub protocol: protocol::ServerOptions,
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

/// Running daemon server bound to an ipc socket.
#[derive(Debug)]
pub struct DaemonServer {
    /// The daemon endpoint metadata.
    endpoint: DaemonEndpoint,
    /// The daemon backing this server.
    daemon: Arc<Daemon>,
    /// Server options in use.
    options: DaemonServerOptions,
    /// Shutdown flag shared across connections.
    shutdown: Arc<AtomicBool>,
}

impl DaemonServer {
    /// Create a new daemon server for a repository and endpoint.
    pub fn new(
        repository: Arc<Repository>,
        endpoint: DaemonEndpoint,
    ) -> Result<Self, DaemonServerError> {
        // use server defaults
        let options = DaemonServerOptions::default();

        // build the server state
        Self::with_options(repository, endpoint, options)
    }

    /// Create a daemon server with explicit options.
    pub fn with_options(
        repository: Arc<Repository>,
        endpoint: DaemonEndpoint,
        options: DaemonServerOptions,
    ) -> Result<Self, DaemonServerError> {
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

    /// Serve daemon requests over an ipc socket until shutdown.
    pub fn serve(&self) -> Result<(), DaemonServerError> {
        // lock the daemon endpoint
        let _lock = self.endpoint.lock().map_err(DaemonServerError::Endpoint)?;

        // clear any stale socket path
        self.endpoint
            .clear_socket_path()
            .map_err(DaemonServerError::Endpoint)?;

        // bind the ipc listener
        let max_frame_bytes = self.options.protocol.limits.max_frame_bytes as usize;
        let listener = DaemonIpcListener::bind(&self.endpoint.socket_path, max_frame_bytes)
            .map_err(DaemonServerError::Ipc)?;

        // write daemon metadata
        let metadata = DaemonMetadata::new(&self.endpoint);
        self.endpoint
            .write_metadata(&metadata)
            .map_err(DaemonServerError::Endpoint)?;

        // serve incoming connections
        let result = self.serve_listener(listener);

        // clean up daemon metadata and socket path
        let _ = self.endpoint.remove_metadata();
        let _ = self.endpoint.clear_socket_path();

        // return the serve result
        result
    }

    /// Serve daemon connections from a listener.
    fn serve_listener(&self, listener: DaemonIpcListener) -> Result<(), DaemonServerError> {
        // initialize connection state
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let activity = Arc::new(protocol::ServerActivity::new(self.options.idle_shutdown));
        let control = protocol::ServerControl::with_activity(self.shutdown.clone(), activity);
        let monitor = self.spawn_idle_monitor(control.clone());

        // accept connections until shutdown
        loop {
            // exit when shutdown is requested
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            // accept the next transport or wait
            let transport = match listener.accept() {
                Ok(transport) => transport,
                Err(DaemonIpcError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    continue;
                }
                Err(error) => return Err(DaemonServerError::Ipc(error)),
            };

            // clone shared state for the connection task
            let daemon = self.daemon.clone();
            let options = self.options.protocol.clone();
            let control = control.clone();

            // spawn a protocol server thread
            let handle = std::thread::spawn(move || {
                let server = protocol::Server::with_control(daemon, options, control);
                let _ = server.serve(transport.as_ref());
            });
            handles.push(handle);
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
    fn spawn_idle_monitor(&self, control: protocol::ServerControl) -> Option<JoinHandle<()>> {
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

                // shut down when the daemon is idle
                if control.should_shutdown() {
                    control.request_shutdown();
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
    /// Endpoint error.
    Endpoint(DaemonEndpointError),
    /// Ipc error.
    Ipc(DaemonIpcError),
    /// Daemon state error.
    Daemon(DaemonError),
}

impl std::fmt::Display for DaemonServerError {
    /// Format the daemon server error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonServerError::Endpoint(error) => {
                write!(f, "daemon server endpoint error: {error}")
            }
            DaemonServerError::Ipc(error) => write!(f, "daemon server ipc error: {error}"),
            DaemonServerError::Daemon(error) => {
                write!(f, "daemon server state error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonServerError {}

impl From<DaemonEndpointError> for DaemonServerError {
    /// Convert an endpoint error into a server error.
    fn from(error: DaemonEndpointError) -> Self {
        DaemonServerError::Endpoint(error)
    }
}

impl From<DaemonIpcError> for DaemonServerError {
    /// Convert an ipc error into a server error.
    fn from(error: DaemonIpcError) -> Self {
        DaemonServerError::Ipc(error)
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
            protocol: protocol::ServerOptions::default(),
            idle_shutdown: Some(Duration::from_millis(DEFAULT_IDLE_SHUTDOWN_MS)),
        }
    }
}
