use std::sync::Arc;
use std::time::Instant;

use destack_rpc::{ConnectError as RpcConnectError, Connection, IpcTransport, Transport};
use destack_workspace::WorkspaceClient;

use super::{DaemonConnectOptions, DaemonEndpoint, DaemonEndpointError, DaemonLaunch};
use crate::ControlClient;

/// Typed connection to one daemon endpoint.
#[derive(Debug)]
pub struct DaemonConnection {
    /// Shared negotiated RPC connection.
    connection: Arc<Connection>,
    /// Workspace operations.
    workspace: WorkspaceClient,
    /// Daemon control operations.
    control: ControlClient,
}

impl DaemonConnection {
    /// Return workspace operations.
    pub const fn workspace(&self) -> &WorkspaceClient {
        &self.workspace
    }

    /// Return daemon control operations.
    pub const fn control(&self) -> &ControlClient {
        &self.control
    }

    /// Close this daemon connection.
    pub fn close(&self) -> Result<(), destack_rpc::ConnectionError> {
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
        let workspace = WorkspaceClient::service_schema().map_err(RpcConnectError::from)?;
        let control = ControlClient::service_schema().map_err(RpcConnectError::from)?;
        let services = vec![workspace.id(), control.id()];
        let connection =
            Connection::connect(transport, options.rpc, services).map_err(RpcConnectError::from)?;
        let workspace = WorkspaceClient::new(connection.clone())?;
        let control = ControlClient::new(connection.clone())?;

        Ok(DaemonConnection {
            connection,
            workspace,
            control,
        })
    }

    /// Start a daemon when absent and verify its RPC endpoint.
    pub fn start(
        &self,
        options: DaemonConnectOptions,
        launch: DaemonLaunch,
    ) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, Some(launch))?;
        connection
            .close()
            .map_err(destack_rpc::ConnectError::from)?;

        Ok(())
    }

    /// Request orderly daemon shutdown.
    pub fn stop(&self, options: DaemonConnectOptions) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, None)?;
        connection.control.shutdown(())?;
        connection
            .close()
            .map_err(destack_rpc::ConnectError::from)?;

        Ok(())
    }

    /// Verify that a daemon accepts a complete RPC connection.
    pub fn ping(&self, options: DaemonConnectOptions) -> Result<(), DaemonEndpointError> {
        let connection = self.connect(options, None)?;
        connection
            .close()
            .map_err(destack_rpc::ConnectError::from)?;

        Ok(())
    }

    /// Wait for a launched daemon transport.
    fn wait_for_transport(
        &self,
        options: &DaemonConnectOptions,
        error: destack_rpc::IpcError,
    ) -> Result<Arc<dyn Transport>, DaemonEndpointError> {
        let deadline = Instant::now() + options.timeout;
        let max_message_bytes = options.rpc.limits.max_message_bytes as usize;
        let mut last_error = error;

        while Instant::now() < deadline {
            std::thread::sleep(options.retry_delay);
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
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.spawn()?;

        Ok(())
    }
}
