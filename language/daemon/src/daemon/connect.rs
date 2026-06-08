use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{DaemonEndpoint, DaemonLaunch};
use crate::ipc::{DaemonIpcError, connect_ipc};
use crate::protocol::{Client, ClientError, ClientOptions, ProtocolLimits, Transport};

/// Connection to a daemon.
#[derive(Debug)]
pub struct DaemonConnection {
    /// Protocol client for daemon requests.
    pub client: Arc<Client>,
}

/// Options for connecting to a daemon.
#[derive(Clone)]
pub struct DaemonConnectOptions {
    /// Client handshake options.
    pub client: ClientOptions,
    /// Protocol limits used while connecting.
    pub limits: ProtocolLimits,
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
            .field("limits", &self.limits)
            .field("retry_delay", &self.retry_delay)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for DaemonConnectOptions {
    /// Return default connect options.
    fn default() -> Self {
        Self {
            client: ClientOptions::default(),
            limits: ProtocolLimits::default(),
            retry_delay: Duration::from_millis(50),
            timeout: Duration::from_secs(3),
        }
    }
}

/// Connect to a daemon over ipc, spawning if needed.
pub fn connect_ipc_daemon(
    endpoint: &DaemonEndpoint,
    options: DaemonConnectOptions,
    launch: Option<DaemonLaunch>,
) -> Result<DaemonConnection, DaemonConnectError> {
    // connect to the daemon or spawn it on demand
    let transport = match connect_ipc(
        &endpoint.socket_path,
        options.limits.max_frame_bytes as usize,
    ) {
        Ok(transport) => transport,
        Err(error) => {
            // spawn the daemon when launch values are provided
            if let Some(launch) = launch {
                spawn_daemon(launch)?;
            } else {
                return Err(DaemonConnectError::Ipc(error));
            }
            // wait for the socket to appear
            wait_for_socket(endpoint, &options, error)?
        }
    };

    // perform the handshake
    let client = Arc::new(Client::new(transport));
    client
        .handshake(options.client)
        .map_err(DaemonConnectError::Client)?;

    Ok(DaemonConnection { client })
}

/// Wait for the daemon socket to appear and connect.
fn wait_for_socket(
    endpoint: &DaemonEndpoint,
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
            &endpoint.socket_path,
            options.limits.max_frame_bytes as usize,
        ) {
            Ok(transport) => return Ok(transport),
            Err(error) => last_error = error,
        }
    }

    Err(DaemonConnectError::Ipc(last_error))
}

/// Spawn a daemon process for launch values.
fn spawn_daemon(launch: DaemonLaunch) -> Result<(), DaemonConnectError> {
    // build the spawn command
    let mut command = launch
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
    Client(ClientError),
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

impl From<ClientError> for DaemonConnectError {
    /// Convert a protocol error into a connect error.
    fn from(error: ClientError) -> Self {
        DaemonConnectError::Client(error)
    }
}
