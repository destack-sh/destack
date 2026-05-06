use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use destack_session::{Session, SessionEventHandler};
use destack_workspace::Repository;

use super::instance::{DaemonInstance, DaemonLaunchConfig};
use crate::ipc::{DaemonIpcError, connect_ipc};
use crate::protocol::{
    DaemonRequest, ProtocolClient, ProtocolClientError, ProtocolClientOptions,
    ProtocolServerOptions, Transport, loopback_transport_pair,
};
use crate::{DaemonService, DaemonServiceError, DaemonServiceOptions};

/// Connection to a daemon with optional in process server handle.
#[derive(Debug)]
pub struct DaemonConnection {
    /// Protocol client for daemon requests.
    pub client: Arc<ProtocolClient>,
    /// Server thread handle for in process daemons.
    server_handle: Option<JoinHandle<Result<(), DaemonServiceError>>>,
}

impl DaemonConnection {
    /// Return true when this connection owns an in process server.
    pub fn is_in_process(&self) -> bool {
        self.server_handle.is_some()
    }
}

impl Drop for DaemonConnection {
    /// Request shutdown for in process servers and join the thread.
    fn drop(&mut self) {
        // shut down the in process server when owned
        if let Some(handle) = self.server_handle.take() {
            let _ = self.client.send_request(DaemonRequest::Shutdown);
            let _ = handle.join();
        }
    }
}

/// Options for connecting to a daemon.
#[derive(Clone)]
pub struct DaemonConnectOptions {
    /// Client handshake options.
    pub client: ProtocolClientOptions,
    /// Server options used for in process connections.
    pub server: ProtocolServerOptions,
    /// Number of workers for each in process session.
    pub worker_limit: usize,
    /// Optional session event handler for in process progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Delay between connection attempts.
    pub retry_delay: Duration,
    /// Maximum time to wait for a daemon.
    pub timeout: Duration,
}

impl std::fmt::Debug for DaemonConnectOptions {
    /// Format the visible daemon connect options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonConnectOptions")
            .field("client", &self.client)
            .field("server", &self.server)
            .field("worker_limit", &self.worker_limit)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field("retry_delay", &self.retry_delay)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for DaemonConnectOptions {
    /// Return default connect options.
    fn default() -> Self {
        Self {
            client: ProtocolClientOptions::default(),
            server: ProtocolServerOptions::default(),
            worker_limit: Session::default_worker_limit(),
            session_event_handler: None,
            retry_delay: Duration::from_millis(50),
            timeout: Duration::from_secs(3),
        }
    }
}

/// Connect to a daemon over ipc, spawning if needed.
pub fn connect_ipc_daemon(
    instance: &DaemonInstance,
    options: DaemonConnectOptions,
    launch: Option<DaemonLaunchConfig>,
) -> Result<DaemonConnection, DaemonConnectError> {
    // connect to the daemon or spawn it on demand
    let transport = match connect_ipc(
        &instance.socket_path,
        options.server.limits.max_frame_bytes as usize,
    ) {
        Ok(transport) => transport,
        Err(error) => {
            // spawn the daemon when a launch config is provided
            if let Some(launch) = launch {
                spawn_daemon(launch)?;
            } else {
                return Err(DaemonConnectError::Ipc(error));
            }
            // wait for the socket to appear
            wait_for_socket(instance, &options, error)?
        }
    };

    // perform the handshake
    let client = Arc::new(ProtocolClient::new(transport));
    client
        .handshake(options.client)
        .map_err(DaemonConnectError::Client)?;

    Ok(DaemonConnection {
        client,
        server_handle: None,
    })
}

/// Connect to an in process daemon using a loopback transport.
pub fn connect_in_process_daemon(
    repository: Arc<Repository>,
    options: DaemonConnectOptions,
) -> Result<DaemonConnection, DaemonConnectError> {
    // build service options
    let service_options = DaemonServiceOptions {
        worker_limit: options.worker_limit,
        session_event_handler: options.session_event_handler.clone(),
        protocol: options.server.clone(),
    };

    // start the in process daemon service
    let service = DaemonService::with_options(repository, service_options);

    // create the loopback transport pair
    let (client_transport, server_transport) = loopback_transport_pair(16);
    let server_handle = std::thread::spawn(move || service.serve_transport(&server_transport));

    // create a protocol client and handshake
    let client = Arc::new(ProtocolClient::new(Arc::new(client_transport)));
    if let Err(error) = client.handshake(options.client) {
        // shutdown the server on failed handshake
        let _ = client.send_request(DaemonRequest::Shutdown);
        let _ = server_handle.join();
        return Err(DaemonConnectError::Client(error));
    }

    Ok(DaemonConnection {
        client,
        server_handle: Some(server_handle),
    })
}

/// Wait for the daemon socket to appear and connect.
fn wait_for_socket(
    instance: &DaemonInstance,
    options: &DaemonConnectOptions,
    error: DaemonIpcError,
) -> Result<Arc<dyn Transport>, DaemonConnectError> {
    // compute the deadline for connection attempts
    let deadline = Instant::now() + options.timeout;
    let mut last_error = error;

    while Instant::now() < deadline {
        // wait before retrying
        std::thread::sleep(options.retry_delay);
        match connect_ipc(
            &instance.socket_path,
            options.server.limits.max_frame_bytes as usize,
        ) {
            Ok(transport) => return Ok(transport),
            Err(error) => last_error = error,
        }
    }

    Err(DaemonConnectError::Ipc(last_error))
}

/// Spawn a daemon process for a launch config.
fn spawn_daemon(config: DaemonLaunchConfig) -> Result<(), DaemonConnectError> {
    // build the spawn command
    let mut command = config
        .command_from_current_exe()
        .map_err(DaemonConnectError::Io)?;

    // detach stdin and output
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // start the daemon process
    command.spawn().map_err(DaemonConnectError::Io)?;
    Ok(())
}

/// Errors for daemon connections.
#[derive(Debug)]
pub enum DaemonConnectError {
    /// Ipc connection error.
    Ipc(DaemonIpcError),
    /// Client protocol error.
    Client(ProtocolClientError),
    /// Spawn error.
    Io(std::io::Error),
}

impl std::fmt::Display for DaemonConnectError {
    /// Format the daemon connect error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonConnectError::Ipc(error) => write!(f, "daemon connect error: {error}"),
            DaemonConnectError::Client(error) => write!(f, "daemon protocol error: {error}"),
            DaemonConnectError::Io(error) => write!(f, "daemon spawn error: {error}"),
        }
    }
}

impl std::error::Error for DaemonConnectError {}

impl From<DaemonIpcError> for DaemonConnectError {
    /// Convert an ipc error into a connect error.
    fn from(error: DaemonIpcError) -> Self {
        DaemonConnectError::Ipc(error)
    }
}

impl From<ProtocolClientError> for DaemonConnectError {
    /// Convert a protocol error into a connect error.
    fn from(error: ProtocolClientError) -> Self {
        DaemonConnectError::Client(error)
    }
}
