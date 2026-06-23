use std::sync::Arc;
use std::time::Instant;

use crate::protocol::{WorkspaceRequest, WorkspaceResponse};
use crate::transport::connect_ipc;
use crate::{Client, ClientError, IpcError, Launch, Service, Transport};

use super::{ConnectError, ConnectOptions};

impl Service {
    /// Connect to the workspace service, spawning if needed.
    pub fn connect(
        &self,
        options: ConnectOptions,
        launch: Option<Launch>,
    ) -> Result<Arc<Client>, ConnectError> {
        // connect to the workspace server or spawn it on demand
        let transport = match self.connect_ipc(&options) {
            Ok(transport) => transport,
            Err(error) => {
                let Some(launch) = launch else {
                    return Err(ConnectError::Transport(error));
                };
                launch.spawn()?;

                self.wait_for_socket(&options, error)?
            }
        };

        // perform the protocol handshake
        let client = Arc::new(Client::new(transport));
        client
            .handshake(options.client)
            .map_err(ConnectError::Client)?;

        Ok(client)
    }

    /// Start the workspace service, spawning if needed.
    pub fn start(&self, options: ConnectOptions, launch: Launch) -> Result<(), ConnectError> {
        let _ = self.connect(options, Some(launch))?;

        Ok(())
    }

    /// Stop a running workspace service.
    pub fn stop(&self, options: ConnectOptions) -> Result<(), ConnectError> {
        let connection = self.connect(options, None)?;
        let response = connection
            .send_request(WorkspaceRequest::Shutdown)
            .map_err(ConnectError::Client)?;

        match response {
            WorkspaceResponse::ShutdownAck => Ok(()),
            WorkspaceResponse::Error(error) => {
                Err(ConnectError::Client(ClientError::Server(error)))
            }
            response => Err(ConnectError::unexpected_response("shutdown", response)),
        }
    }

    /// Ping a running workspace service.
    pub fn ping(&self, options: ConnectOptions) -> Result<(), ConnectError> {
        let connection = self.connect(options, None)?;
        let response = connection
            .send_request(WorkspaceRequest::Ping)
            .map_err(ConnectError::Client)?;

        match response {
            WorkspaceResponse::Pong => Ok(()),
            WorkspaceResponse::Error(error) => {
                Err(ConnectError::Client(ClientError::Server(error)))
            }
            response => Err(ConnectError::unexpected_response("ping", response)),
        }
    }

    /// Connect to the service socket.
    fn connect_ipc(&self, options: &ConnectOptions) -> Result<Arc<dyn Transport>, IpcError> {
        connect_ipc(&self.socket_path, options.limits.max_frame_bytes as usize)
    }

    /// Wait for the workspace service socket to appear and connect.
    fn wait_for_socket(
        &self,
        options: &ConnectOptions,
        error: IpcError,
    ) -> Result<Arc<dyn Transport>, ConnectError> {
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

        Err(ConnectError::Transport(last_error))
    }
}

impl Launch {
    /// Spawn a workspace service process.
    fn spawn(self) -> Result<(), ConnectError> {
        // build the spawn command
        let mut command = self.command();

        // detach stdin and output
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        // start the workspace server process
        command.spawn().map_err(ConnectError::Io)?;
        Ok(())
    }
}
