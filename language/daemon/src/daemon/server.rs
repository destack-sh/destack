use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use std::time::Duration;

use destack_repository::Repository;
use destack_rpc::{
    ConnectionOptions, IpcListener, Registry, ServerError, Transport, WebSocketError,
    WebSocketListener,
};
use destack_session::{Session, SessionEventHandler};
use destack_workspace::{SharedWorkspace, WorkspaceServer};

use super::constants::{DEFAULT_IDLE_SHUTDOWN_MS, IDLE_SHUTDOWN_POLL_MS};
use crate::{
    Control, ControlServer, Daemon, DaemonActivity, DaemonEndpoint, DaemonEndpointError,
    DaemonError, DaemonLifecycle, DaemonMetadata, OpenedWorkspace,
};

/// Options for serving daemon RPC connections.
#[derive(Clone)]
pub struct DaemonServerOptions {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for daemon progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// RPC negotiation and resource options.
    pub rpc: ConnectionOptions,
    /// Idle shutdown timeout.
    pub idle_shutdown: Option<Duration>,
}

impl std::fmt::Debug for DaemonServerOptions {
    /// Format visible daemon server options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonServerOptions")
            .field("worker_limit", &self.worker_limit)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field("rpc", &self.rpc)
            .field("idle_shutdown", &self.idle_shutdown)
            .finish()
    }
}

/// Daemon process serving workspace and control RPC services.
#[derive(Debug)]
pub struct DaemonServer {
    /// Discovery and transport addresses.
    endpoint: DaemonEndpoint,
    /// Workspace exposed by this daemon.
    workspace: Arc<OpenedWorkspace>,
    /// Server options.
    options: DaemonServerOptions,
    /// Shared process lifecycle.
    lifecycle: DaemonLifecycle,
}

impl DaemonServer {
    /// Create a daemon server with default options.
    pub fn new(
        repository: Arc<Repository>,
        endpoint: DaemonEndpoint,
    ) -> Result<Self, DaemonServerError> {
        Self::with_options(repository, endpoint, DaemonServerOptions::default())
    }

    /// Create a daemon server with explicit options.
    pub fn with_options(
        repository: Arc<Repository>,
        endpoint: DaemonEndpoint,
        options: DaemonServerOptions,
    ) -> Result<Self, DaemonServerError> {
        let workspace_root = repository.path().to_path_buf();
        let daemon = Daemon::new(
            repository,
            options.worker_limit,
            options.session_event_handler.clone(),
        )?;
        let workspace = daemon.open(&workspace_root)?;
        let activity = Arc::new(DaemonActivity::new(options.idle_shutdown));
        let lifecycle = DaemonLifecycle::new(Arc::new(AtomicBool::new(false)), activity);

        Ok(Self {
            endpoint,
            workspace,
            options,
            lifecycle,
        })
    }

    /// Serve daemon connections until shutdown.
    pub fn serve(&self) -> Result<(), DaemonServerError> {
        let _lock = self.endpoint.lock()?;
        self.endpoint.clear_socket()?;
        let websocket_token = self.endpoint.websocket_token()?;
        let max_message_bytes = self.options.rpc.limits.max_message_bytes as usize;
        let ipc = IpcListener::bind(&self.endpoint.socket_path, max_message_bytes)?;
        let websocket = WebSocketListener::bind(
            self.endpoint.websocket_address,
            max_message_bytes,
            self.endpoint.websocket_path.clone(),
            websocket_token.clone(),
        )?;
        let websocket_address = websocket.local_address()?;
        let metadata = DaemonMetadata::new(&self.endpoint, websocket_address, &websocket_token)?;
        self.endpoint.write_metadata(&metadata)?;

        let result = self.serve_connections(ipc, websocket);
        let metadata_result = self.endpoint.remove_metadata();
        let socket_result = self.endpoint.clear_socket();

        result?;
        metadata_result?;
        socket_result?;

        Ok(())
    }

    /// Accept and serve both local and browser transports.
    fn serve_connections(
        &self,
        ipc: IpcListener,
        websocket: WebSocketListener,
    ) -> Result<(), DaemonServerError> {
        let monitor = self.spawn_idle_monitor();
        let mut connections = Vec::new();

        while !self.lifecycle.is_shutdown() {
            let ipc_transport = ipc.try_accept()?;
            let websocket_transport = websocket.try_accept()?;
            let accepted = ipc_transport.into_iter().chain(websocket_transport);
            let mut accepted_any = false;

            for transport in accepted {
                accepted_any = true;
                connections.push(self.spawn_connection(transport)?);
            }
            Self::reap_connections(&mut connections)?;

            if !accepted_any {
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        // allow the shutdown caller to receive its terminal response and disconnect
        while !connections.is_empty() && !connections.iter().any(ConnectionTask::is_finished) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Self::reap_connections(&mut connections)?;
        for connection in &connections {
            connection.transport.close()?;
        }
        Self::join_connections(connections)?;

        if let Some(monitor) = monitor {
            monitor.join().map_err(|_| DaemonServerError::Thread)?;
        }

        Ok(())
    }

    /// Spawn one RPC connection.
    fn spawn_connection(
        &self,
        transport: Arc<dyn Transport>,
    ) -> Result<ConnectionTask, DaemonServerError> {
        let server = self.rpc_server()?;
        let lifecycle = self.lifecycle.clone();
        let connection_transport = transport.clone();
        lifecycle.connect();
        let handle = std::thread::spawn(move || {
            let result = server.serve(connection_transport);
            lifecycle.disconnect();

            result
        });

        Ok(ConnectionTask { transport, handle })
    }

    /// Build connection-scoped RPC services.
    fn rpc_server(&self) -> Result<destack_rpc::Server, DaemonServerError> {
        let workspace = SharedWorkspace::new(self.workspace.workspace());
        let workspace = WorkspaceServer::new(workspace)?;
        let control = ControlServer::new(Control::new(self.lifecycle.clone()))?;
        let mut services = Registry::new();
        services.insert(workspace)?;
        services.insert(control)?;

        Ok(destack_rpc::Server::new(services, self.options.rpc.clone()))
    }

    /// Spawn idle shutdown monitoring when configured.
    fn spawn_idle_monitor(&self) -> Option<JoinHandle<()>> {
        self.options.idle_shutdown?;
        let lifecycle = self.lifecycle.clone();
        let interval = Duration::from_millis(IDLE_SHUTDOWN_POLL_MS);

        Some(std::thread::spawn(move || {
            while !lifecycle.is_shutdown() {
                if lifecycle.shutdown_if_idle() {
                    return;
                }
                std::thread::sleep(interval);
            }
        }))
    }

    /// Join and remove completed connections.
    fn reap_connections(connections: &mut Vec<ConnectionTask>) -> Result<(), DaemonServerError> {
        let mut index = 0;

        while index < connections.len() {
            if connections[index].is_finished() {
                let connection = connections.swap_remove(index);
                connection.join()?;
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    /// Join every remaining connection.
    fn join_connections(connections: Vec<ConnectionTask>) -> Result<(), DaemonServerError> {
        for connection in connections {
            connection.join()?;
        }

        Ok(())
    }
}

/// One live daemon RPC connection.
#[derive(Debug)]
struct ConnectionTask {
    /// Transport used to interrupt the connection during shutdown.
    transport: Arc<dyn Transport>,
    /// Connection server thread.
    handle: JoinHandle<Result<(), ServerError>>,
}

impl ConnectionTask {
    /// Return whether this connection thread terminated.
    fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Join this connection and propagate its failure.
    fn join(self) -> Result<(), DaemonServerError> {
        self.handle
            .join()
            .map_err(|_| DaemonServerError::Thread)??;

        Ok(())
    }
}

/// Failure while serving daemon connections.
#[derive(Debug)]
pub enum DaemonServerError {
    /// Daemon repository state failed.
    Daemon(DaemonError),
    /// Endpoint discovery or local transport failed.
    Endpoint(DaemonEndpointError),
    /// WebSocket listener failed.
    WebSocket(WebSocketError),
    /// RPC service declaration failed.
    Schema(destack_rpc::ServiceSchemaError),
    /// RPC service registration failed.
    Registry(destack_rpc::RegistryError),
    /// RPC connection failed.
    Connection(ServerError),
    /// A daemon thread panicked.
    Thread,
}

impl std::fmt::Display for DaemonServerError {
    /// Format this daemon server failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daemon(error) => write!(formatter, "daemon state failed: {error}"),
            Self::Endpoint(error) => write!(formatter, "{error}"),
            Self::WebSocket(error) => write!(formatter, "daemon WebSocket failed: {error}"),
            Self::Schema(error) => write!(formatter, "daemon RPC schema failed: {error}"),
            Self::Registry(error) => write!(formatter, "daemon RPC registry failed: {error}"),
            Self::Connection(error) => write!(formatter, "daemon RPC connection failed: {error}"),
            Self::Thread => write!(formatter, "daemon thread panicked"),
        }
    }
}

impl std::error::Error for DaemonServerError {
    /// Return the underlying daemon server failure when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Daemon(error) => Some(error),
            Self::Endpoint(error) => Some(error),
            Self::WebSocket(error) => Some(error),
            Self::Schema(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::Connection(error) => Some(error),
            Self::Thread => None,
        }
    }
}

impl From<DaemonError> for DaemonServerError {
    /// Convert one daemon state failure.
    fn from(error: DaemonError) -> Self {
        Self::Daemon(error)
    }
}

impl From<DaemonEndpointError> for DaemonServerError {
    /// Convert one endpoint failure.
    fn from(error: DaemonEndpointError) -> Self {
        Self::Endpoint(error)
    }
}

impl From<destack_rpc::IpcError> for DaemonServerError {
    /// Convert one local transport failure.
    fn from(error: destack_rpc::IpcError) -> Self {
        Self::Endpoint(error.into())
    }
}

impl From<destack_rpc::TransportError> for DaemonServerError {
    /// Convert one established transport failure.
    fn from(error: destack_rpc::TransportError) -> Self {
        Self::Connection(destack_rpc::ConnectionError::from(error).into())
    }
}

impl From<WebSocketError> for DaemonServerError {
    /// Convert one WebSocket listener failure.
    fn from(error: WebSocketError) -> Self {
        Self::WebSocket(error)
    }
}

impl From<destack_rpc::ServiceSchemaError> for DaemonServerError {
    /// Convert one service declaration failure.
    fn from(error: destack_rpc::ServiceSchemaError) -> Self {
        Self::Schema(error)
    }
}

impl From<destack_rpc::RegistryError> for DaemonServerError {
    /// Convert one service registration failure.
    fn from(error: destack_rpc::RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<ServerError> for DaemonServerError {
    /// Convert one RPC connection failure.
    fn from(error: ServerError) -> Self {
        Self::Connection(error)
    }
}

impl Default for DaemonServerOptions {
    /// Create default daemon server options.
    fn default() -> Self {
        Self {
            worker_limit: Session::default_worker_count(),
            session_event_handler: None,
            rpc: ConnectionOptions::new("destack-daemon"),
            idle_shutdown: Some(Duration::from_millis(DEFAULT_IDLE_SHUTDOWN_MS)),
        }
    }
}
