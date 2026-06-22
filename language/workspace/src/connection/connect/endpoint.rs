use std::sync::Arc;
use std::time::Instant;

use super::super::ipc::{WorkspaceIpcError, connect_ipc};
use super::super::{WorkspaceEndpoint, WorkspaceLaunch};
use crate::connection::{Client, Transport};
use crate::protocol::{WorkspaceRequest, WorkspaceResponse};
use crate::{ClientError, RemoteWorkspace};

use super::{WorkspaceConnectError, WorkspaceConnectOptions, WorkspaceConnection};

impl WorkspaceEndpoint {
    /// Connect to the workspace endpoint, spawning if needed.
    pub fn connect(
        &self,
        options: WorkspaceConnectOptions,
        launch: Option<WorkspaceLaunch>,
    ) -> Result<WorkspaceConnection, WorkspaceConnectError> {
        // connect to the workspace server or spawn it on demand
        let transport = match self.connect_ipc(&options) {
            Ok(transport) => transport,
            Err(error) => {
                let Some(launch) = launch else {
                    return Err(WorkspaceConnectError::Transport(error));
                };
                launch.spawn()?;

                self.wait_for_socket(&options, error)?
            }
        };

        // perform the protocol handshake
        let client = Arc::new(Client::new(transport));
        client
            .handshake(options.client)
            .map_err(WorkspaceConnectError::Client)?;

        Ok(WorkspaceConnection { client })
    }

    /// Start the workspace endpoint, spawning if needed.
    pub fn start(
        &self,
        options: WorkspaceConnectOptions,
        launch: WorkspaceLaunch,
    ) -> Result<(), WorkspaceConnectError> {
        let _ = self.connect(options, Some(launch))?;

        Ok(())
    }

    /// Stop a running workspace endpoint.
    pub fn stop(&self, options: WorkspaceConnectOptions) -> Result<(), WorkspaceConnectError> {
        let connection = self.connect(options, None)?;
        let response = connection
            .client
            .send_request(WorkspaceRequest::Shutdown)
            .map_err(WorkspaceConnectError::Client)?;

        match response {
            WorkspaceResponse::ShutdownAck => Ok(()),
            WorkspaceResponse::Error(error) => {
                Err(WorkspaceConnectError::Client(ClientError::Server(error)))
            }
            response => Err(WorkspaceConnectError::unexpected_response(
                "shutdown", response,
            )),
        }
    }

    /// Ping a running workspace endpoint.
    pub fn ping(&self, options: WorkspaceConnectOptions) -> Result<(), WorkspaceConnectError> {
        let connection = self.connect(options, None)?;
        let response = connection
            .client
            .send_request(WorkspaceRequest::Ping)
            .map_err(WorkspaceConnectError::Client)?;

        match response {
            WorkspaceResponse::Pong => Ok(()),
            WorkspaceResponse::Error(error) => {
                Err(WorkspaceConnectError::Client(ClientError::Server(error)))
            }
            response => Err(WorkspaceConnectError::unexpected_response("ping", response)),
        }
    }

    /// Connect to a remote workspace, spawning if needed.
    pub fn remote(
        &self,
        options: WorkspaceConnectOptions,
        launch: WorkspaceLaunch,
    ) -> Result<Arc<RemoteWorkspace>, WorkspaceConnectError> {
        let root = launch.root.clone();
        let connection = self.connect(options, Some(launch))?;
        let workspace = Arc::new(RemoteWorkspace::new(connection.client.clone(), root));

        Ok(workspace)
    }

    /// Connect to the endpoint socket.
    fn connect_ipc(
        &self,
        options: &WorkspaceConnectOptions,
    ) -> Result<Arc<dyn Transport>, WorkspaceIpcError> {
        connect_ipc(&self.socket_path, options.limits.max_frame_bytes as usize)
    }

    /// Wait for the workspace server socket to appear and connect.
    fn wait_for_socket(
        &self,
        options: &WorkspaceConnectOptions,
        error: WorkspaceIpcError,
    ) -> Result<Arc<dyn Transport>, WorkspaceConnectError> {
        // compute the deadline for connection attempts
        let deadline = Instant::now() + options.timeout;
        let mut last_error = error;

        while Instant::now() < deadline {
            // wait before retrying
            std::thread::sleep(options.retry_delay);
            match self.connect_ipc(options) {
                Ok(transport) => return Ok(transport),
                Err(error) => last_error = error,
            }
        }

        Err(WorkspaceConnectError::Transport(last_error))
    }
}

impl WorkspaceLaunch {
    /// Spawn a workspace server process.
    fn spawn(self) -> Result<(), WorkspaceConnectError> {
        // build the spawn command
        let mut command = self.command();

        // detach stdin and output
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        // start the workspace server process
        command.spawn().map_err(WorkspaceConnectError::Io)?;
        Ok(())
    }
}
