use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use tspp_rpc::{ConnectError, Connection, ConnectionError, IpcError, IpcTransport, Transport};
use tspp_runtime::service::{DebuggerClient, WorldClient};
use tspp_workspace::WorkspaceClient;

use super::{DaemonConnectOptions, DaemonEndpoint, DaemonEndpointError, DaemonLaunch};
use crate::{BlobClient, DaemonClient};

/// Typed connection to one daemon endpoint.
#[derive(Debug)]
pub struct DaemonConnection {
    /// Shared negotiated RPC connection.
    connection: Arc<Connection>,

    /// Daemon operations.
    daemon: DaemonClient,

    /// Immutable Blob operations.
    blob: BlobClient,
    /// Workspace operations.
    workspace: WorkspaceClient,
    /// World execution operations.
    world: WorldClient,
    /// World debugging operations.
    debugger: DebuggerClient,
}

impl DaemonConnection {
    /// Return daemon operations.
    pub const fn daemon(&self) -> &DaemonClient {
        &self.daemon
    }

    /// Return immutable Blob operations.
    pub const fn blob(&self) -> &BlobClient {
        &self.blob
    }

    /// Return workspace operations.
    pub const fn workspace(&self) -> &WorkspaceClient {
        &self.workspace
    }

    /// Return World execution operations.
    pub const fn world(&self) -> &WorldClient {
        &self.world
    }

    /// Return World debugging operations.
    pub const fn debugger(&self) -> &DebuggerClient {
        &self.debugger
    }

    /// Close this daemon connection.
    pub fn close(&self) -> Result<(), ConnectionError> {
        self.connection.close()
    }
}

impl DaemonEndpoint {
    /// Connect to a daemon, optionally launching it when absent.
    pub fn connect(
        &self,
        options: DaemonConnectOptions,
        launch: Option<DaemonLaunch>,
    ) -> Result<DaemonConnection, DaemonEndpointError> {
        // connect to the published transport or launch its daemon
        let max_message_bytes = options.rpc.limits.max_message_bytes as usize;
        let transport = match IpcTransport::connect(&self.socket_path, max_message_bytes) {
            Ok(transport) => transport,
            Err(error) => {
                let Some(launch) = launch else {
                    return Err(error.into());
                };
                launch.spawn()?;

                self.wait_for_transport(&options, error)?
            }
        };

        // resolve every required service schema
        let daemon = DaemonClient::service_schema().map_err(ConnectError::from)?;

        let blob = BlobClient::service_schema().map_err(ConnectError::from)?;
        let workspace = WorkspaceClient::service_schema().map_err(ConnectError::from)?;
        let world = WorldClient::service_schema().map_err(ConnectError::from)?;
        let debugger = DebuggerClient::service_schema().map_err(ConnectError::from)?;

        // negotiate one connection for all services
        let services = vec![
            daemon.id(),
            blob.id(),
            workspace.id(),
            world.id(),
            debugger.id(),
        ];
        let connection =
            Connection::connect(transport, options.rpc, services).map_err(ConnectError::from)?;

        // bind each typed client to the shared connection
        let daemon = DaemonClient::new(connection.clone())?;

        let blob = BlobClient::new(connection.clone())?;
        let workspace = WorkspaceClient::new(connection.clone())?;
        let world = WorldClient::new(connection.clone())?;
        let debugger = DebuggerClient::new(connection.clone())?;

        Ok(DaemonConnection {
            connection,

            daemon,

            blob,
            workspace,
            world,
            debugger,
        })
    }

    /// Start a daemon when absent and verify its RPC endpoint.
    pub fn start(
        &self,
        options: DaemonConnectOptions,
        launch: DaemonLaunch,
    ) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, Some(launch))?;
        connection.close().map_err(ConnectError::from)?;

        Ok(())
    }

    /// Request orderly daemon shutdown.
    pub fn stop(&self, options: DaemonConnectOptions) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, None)?;
        connection.daemon.shutdown(())?;
        connection.close().map_err(ConnectError::from)?;

        Ok(())
    }

    /// Verify that a daemon accepts a complete RPC connection.
    pub fn ping(&self, options: DaemonConnectOptions) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, None)?;
        connection.close().map_err(ConnectError::from)?;

        Ok(())
    }

    /// Wait for a launched daemon transport.
    fn wait_for_transport(
        &self,
        options: &DaemonConnectOptions,
        error: IpcError,
    ) -> Result<Arc<dyn Transport>, DaemonEndpointError> {
        let deadline = Instant::now() + options.timeout;
        let max_message_bytes = options.rpc.limits.max_message_bytes as usize;
        let mut last_error = error;

        while Instant::now() < deadline {
            thread::sleep(options.retry_delay);
            match IpcTransport::connect(&self.socket_path, max_message_bytes) {
                Ok(transport) => return Ok(transport),
                Err(error) => last_error = error,
            }
        }

        Err(last_error.into())
    }
}

impl DaemonLaunch {
    /// Spawn this daemon process detached from the caller's streams.
    fn spawn(self) -> Result<(), DaemonEndpointError> {
        let mut command = self.command();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn()?;

        Ok(())
    }
}
