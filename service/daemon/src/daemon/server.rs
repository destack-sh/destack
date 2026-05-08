use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use destack_session::{Session, SessionEventHandler};
use destack_workspace::Repository;

use super::instance::{DaemonInstance, DaemonInstanceError, DaemonMetadata};
use crate::ipc::{DaemonIpcError, DaemonIpcListener};
use crate::protocol::{
    ProtocolServer, ProtocolServerActivity, ProtocolServerControl, ProtocolServerError,
    ProtocolServerOptions,
};
use crate::{Daemon, DaemonServiceOptions};

/// Default idle shutdown timeout for the daemon.
pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_MS: u64 = 600_000;

/// Idle shutdown monitor poll interval.
const IDLE_SHUTDOWN_POLL_MS: u64 = 250;

/// Options for the daemon server.
#[derive(Clone)]
pub struct DaemonServerOptions {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for in process progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Protocol options for daemon connections.
    pub protocol: ProtocolServerOptions,
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
    /// The daemon instance metadata.
    instance: DaemonInstance,
    /// The daemon backing this server.
    daemon: Arc<Daemon>,
    /// Server options in use.
    options: DaemonServerOptions,
    /// Shutdown flag shared across connections.
    shutdown: Arc<AtomicBool>,
}

impl DaemonServer {
    /// Create a new daemon server for a repository and instance.
    pub fn new(repository: Arc<Repository>, instance: DaemonInstance) -> Self {
        // use server defaults
        let options = DaemonServerOptions::default();

        // build the server state
        Self::with_options(repository, instance, options)
    }

    /// Create a daemon server with explicit options.
    pub fn with_options(
        repository: Arc<Repository>,
        instance: DaemonInstance,
        options: DaemonServerOptions,
    ) -> Self {
        // build the daemon instance
        let daemon = Arc::new(Daemon::new(
            repository,
            options.worker_limit,
            options.session_event_handler.clone(),
        ));

        // return the server state
        Self {
            instance,
            daemon,
            options,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Serve daemon requests over an ipc socket until shutdown.
    pub fn serve(&self) -> Result<(), DaemonServerError> {
        // lock the daemon instance
        let _lock = self.instance.lock().map_err(DaemonServerError::Instance)?;

        // clear any stale socket path
        self.instance
            .clear_socket_path()
            .map_err(DaemonServerError::Instance)?;

        // bind the ipc listener
        let max_frame_bytes = self.options.protocol.limits.max_frame_bytes as usize;
        let listener = DaemonIpcListener::bind(&self.instance.socket_path, max_frame_bytes)
            .map_err(DaemonServerError::Ipc)?;

        // write daemon metadata
        let metadata = DaemonMetadata::new(&self.instance);
        self.instance
            .write_metadata(&metadata)
            .map_err(DaemonServerError::Instance)?;

        // serve incoming connections
        let result = self.serve_listener(listener);

        // clean up daemon metadata and socket path
        let _ = self.instance.remove_metadata();
        let _ = self.instance.clear_socket_path();

        // return the serve result
        result
    }

    /// Serve daemon connections from a listener.
    fn serve_listener(&self, listener: DaemonIpcListener) -> Result<(), DaemonServerError> {
        // initialize connection state
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let activity = Arc::new(ProtocolServerActivity::new(self.options.idle_shutdown));
        let control = ProtocolServerControl::with_activity(self.shutdown.clone(), activity);
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
                let server = ProtocolServer::with_control(daemon, options, control);
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
    fn spawn_idle_monitor(&self, control: ProtocolServerControl) -> Option<JoinHandle<()>> {
        // return early when idle shutdown is disabled
        let _idle_shutdown = self.options.idle_shutdown?;

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
    /// Instance error.
    Instance(DaemonInstanceError),
    /// Ipc error.
    Ipc(DaemonIpcError),
    /// Protocol error.
    Protocol(ProtocolServerError),
}

impl std::fmt::Display for DaemonServerError {
    /// Format the daemon server error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonServerError::Instance(error) => {
                write!(f, "daemon server instance error: {error}")
            }
            DaemonServerError::Ipc(error) => write!(f, "daemon server ipc error: {error}"),
            DaemonServerError::Protocol(error) => {
                write!(f, "daemon server protocol error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonServerError {}

impl From<DaemonInstanceError> for DaemonServerError {
    /// Convert an instance error into a server error.
    fn from(error: DaemonInstanceError) -> Self {
        DaemonServerError::Instance(error)
    }
}

impl From<DaemonIpcError> for DaemonServerError {
    /// Convert an ipc error into a server error.
    fn from(error: DaemonIpcError) -> Self {
        DaemonServerError::Ipc(error)
    }
}

impl From<ProtocolServerError> for DaemonServerError {
    /// Convert a protocol error into a server error.
    fn from(error: ProtocolServerError) -> Self {
        DaemonServerError::Protocol(error)
    }
}

impl From<DaemonServiceOptions> for DaemonServerOptions {
    /// Convert service options into server options.
    fn from(options: DaemonServiceOptions) -> Self {
        Self {
            worker_limit: options.worker_limit,
            session_event_handler: options.session_event_handler,
            protocol: options.protocol,
            idle_shutdown: Some(Duration::from_millis(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS)),
        }
    }
}

impl Default for DaemonServerOptions {
    /// Create default daemon server options.
    fn default() -> Self {
        Self {
            worker_limit: Session::default_worker_limit(),
            session_event_handler: None,
            protocol: ProtocolServerOptions::default(),
            idle_shutdown: Some(Duration::from_millis(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS)),
        }
    }
}
