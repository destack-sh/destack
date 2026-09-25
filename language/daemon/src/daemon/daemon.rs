use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, select, unbounded};
use tspp_core::BlobStore;
use tspp_rpc::{
    IpcListener, Listener, Registry, Server, ServerError, Transport, TransportError,
    WebSocketListener,
};
use tspp_runtime::service::{DebuggerServer, HostServer, WorldServer};
use tspp_workspace::{Workspace, WorkspaceServer};

use crate::{
    BlobServer, ConnectionActivity, ConnectionId, DaemonEndpoint, DaemonError, DaemonLifecycle,
    DaemonMetadata, DaemonOptions, DaemonPeer, DaemonServer, WorkspaceRegistry, WorldRegistry,
};

/// Persistent process serving TS++ RPC services.
#[derive(Debug, Clone)]
pub struct Daemon {
    /// Discovery and transport addresses.
    endpoint: DaemonEndpoint,
    /// Immutable bytes shared by daemon services.
    blobs: Arc<BlobStore>,
    /// Root-bound workspaces exposed by this daemon.
    workspaces: WorkspaceRegistry,
    /// Runtime Worlds exposed by this daemon.
    worlds: WorldRegistry,
    /// Daemon options.
    options: DaemonOptions,
    /// Shared process lifecycle.
    lifecycle: DaemonLifecycle,
}

impl Daemon {
    /// Create a daemon with default options.
    pub fn new(workspace: Workspace, endpoint: DaemonEndpoint) -> Result<Self, DaemonError> {
        Self::with_options(workspace, endpoint, DaemonOptions::default())
    }

    /// Create a daemon with explicit options.
    pub fn with_options(
        workspace: Workspace,
        endpoint: DaemonEndpoint,
        options: DaemonOptions,
    ) -> Result<Self, DaemonError> {
        // retain one BlobStore across every daemon service and workspace
        let blobs = workspace.session().repository().blob_store().clone();

        // host shared workspace state and physical changes
        let workspaces = WorkspaceRegistry::new(workspace)?;

        // host runtime Worlds over the same immutable Blob storage
        let worlds = WorldRegistry::new(blobs.clone());

        // create process lifecycle state
        let lifecycle = DaemonLifecycle::new(options.idle_timeout);

        Ok(Self {
            endpoint,
            blobs,
            workspaces,
            worlds,
            options,
            lifecycle,
        })
    }

    /// Serve daemon connections until shutdown.
    pub fn serve(&self) -> Result<(), DaemonError> {
        // acquire the endpoint and remove stale local state
        let _lock = self.endpoint.lock()?;
        self.endpoint.remove_metadata()?;
        self.endpoint.clear_socket()?;

        // remove published endpoint state after every listening outcome
        let result = self.listen();
        let workspace_result = self.workspaces.close_all();
        self.worlds.close_all();
        let metadata_result = self.endpoint.remove_metadata();
        let socket_result = self.endpoint.clear_socket();

        // preserve serving and endpoint cleanup failures together
        let failures = [
            result,
            workspace_result,
            metadata_result.map_err(DaemonError::from),
            socket_result.map_err(DaemonError::from),
        ]
        .into_iter()
        .filter_map(Result::err)
        .collect();

        match DaemonError::combine(failures) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Bind transports, publish the endpoint, and serve connections.
    fn listen(&self) -> Result<(), DaemonError> {
        // bind local and browser transports
        let websocket_token = self.endpoint.websocket_token()?;
        let max_message_bytes = self.options.rpc.limits.max_message_bytes as usize;
        let ipc = IpcListener::bind(&self.endpoint.socket_path, max_message_bytes)?;
        let websocket = WebSocketListener::bind(
            self.endpoint.websocket_address,
            max_message_bytes,
            self.endpoint.websocket_path.clone(),
            websocket_token.clone(),
        )?;

        // publish the live endpoint
        let websocket_address = websocket.local_address();
        let metadata = DaemonMetadata::new(
            &self.endpoint,
            websocket_address,
            &websocket_token,
            self.workspaces.build_id(),
        )?;
        self.endpoint.write_metadata(&metadata)?;

        self.serve_connections(ipc, websocket)
    }

    /// Accept and serve both local and browser transports.
    fn serve_connections(
        &self,
        ipc: IpcListener,
        websocket: WebSocketListener,
    ) -> Result<(), DaemonError> {
        let ipc = Arc::new(ipc);
        let websocket = Arc::new(websocket);
        let (sender, events) = unbounded();

        // block independently on both RPC listeners
        let ipc_thread = Self::spawn_listener(ipc.clone(), sender.clone());
        let websocket_thread = Self::spawn_listener(websocket.clone(), sender.clone());
        drop(sender);

        // serve accepted connections until shutdown or failure
        let mut connections = Vec::new();
        let serve_result = self.accept_connections(&events, &mut connections);
        self.lifecycle.shutdown();

        // interrupt and join both RPC listeners
        let mut failures = Vec::new();
        failures.extend(ipc.close().err().map(DaemonError::from));
        failures.extend(websocket.close().err().map(DaemonError::from));
        failures.extend(Self::join_listener(ipc_thread).err());
        failures.extend(Self::join_listener(websocket_thread).err());
        failures.extend(events.try_iter().filter_map(|event| event.finish().err()));

        // let the shutdown caller receive its terminal response and disconnect
        let completion_result = if serve_result.is_ok() {
            self.wait_for_shutdown_connection(&mut connections)
        } else {
            Ok(())
        };

        // interrupt and join every remaining connection
        let reap_result = Connection::reap(&mut connections);
        failures.extend(
            connections
                .iter()
                .filter_map(|connection| connection.close().err().map(DaemonError::from)),
        );
        failures.extend(
            connections
                .into_iter()
                .filter_map(|connection| connection.join().err()),
        );

        // preserve every independently observed serving and cleanup failure
        failures.extend(serve_result.err());
        failures.extend(completion_result.err());
        failures.extend(reap_result.err());

        match DaemonError::combine(failures) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Accept connections until lifecycle shutdown or one serving failure.
    fn accept_connections(
        &self,
        events: &Receiver<Event>,
        connections: &mut Vec<Connection>,
    ) -> Result<(), DaemonError> {
        while !self.lifecycle.is_shutdown() {
            match self.next_event(events)? {
                Some(Event::Accepted(transport)) => {
                    let activity = self.lifecycle.connect();
                    let server = self.rpc_server(activity.identifier())?;
                    connections.push(Connection::spawn(server, transport, activity));
                }
                Some(Event::Failed(error)) => return Err(error),
                None => {}
            }

            // apply current lifecycle state and completed connection results
            self.lifecycle.shutdown_if_idle();
            Connection::reap(connections)?;
        }

        Ok(())
    }

    /// Wait for one daemon or listener transition.
    fn next_event(&self, events: &Receiver<Event>) -> Result<Option<Event>, DaemonError> {
        let notifications = self.lifecycle.notifications();
        let Some(deadline) = self.lifecycle.idle_deadline() else {
            return select! {
                recv(events) -> event => Event::receive(event),
                recv(notifications) -> notification => {
                    notification.map_err(|_| DaemonError::Thread)?;

                    Ok(None)
                }
            };
        };
        let timeout = deadline.saturating_duration_since(Instant::now());

        select! {
            recv(events) -> event => Event::receive(event),
            recv(notifications) -> notification => {
                notification.map_err(|_| DaemonError::Thread)?;

                Ok(None)
            }
            default(timeout) => Ok(None),
        }
    }

    /// Spawn one blocking RPC listener.
    fn spawn_listener<L>(
        listener: Arc<L>,
        sender: Sender<Event>,
    ) -> JoinHandle<Result<(), DaemonError>>
    where
        L: Listener,
        DaemonError: From<L::Error>,
    {
        thread::spawn(move || {
            loop {
                match listener.accept() {
                    Ok(Some(transport)) => {
                        let sent = sender.send(Event::Accepted(transport));
                        if let Err(error) = sent {
                            let Event::Accepted(transport) = error.0 else {
                                return Err(DaemonError::Thread);
                            };

                            transport.close()?;

                            return Ok(());
                        }
                    }
                    Ok(None) => return Ok(()),
                    Err(error) => {
                        let sent = sender.send(Event::Failed(error.into()));
                        if let Err(error) = sent {
                            let Event::Failed(error) = error.0 else {
                                return Err(DaemonError::Thread);
                            };

                            return Err(error);
                        }

                        return Ok(());
                    }
                }
            }
        })
    }

    /// Build connection-scoped RPC services.
    fn rpc_server(&self, connection: ConnectionId) -> Result<Server, DaemonError> {
        // bind daemon control and workspace access to one connection
        let peer = DaemonPeer::new(self.clone(), connection);
        let daemon = DaemonServer::new(peer.clone())?;
        let workspace = WorkspaceServer::new(peer)?;

        // bind process services to their shared owners
        let blob = BlobServer::new(self.blobs.clone())?;
        let world = WorldServer::new(self.worlds.clone())?;
        let host = HostServer::new(self.worlds.clone())?;
        let debugger = DebuggerServer::new(self.worlds.clone())?;

        // register connection services
        let mut services = Registry::new();
        services.insert(daemon)?;
        services.insert(workspace)?;

        // register shared process services
        services.insert(blob)?;
        services.insert(world)?;
        services.insert(host)?;
        services.insert(debugger)?;

        Ok(Server::new(services, self.options.rpc.clone()))
    }

    /// Wait until the connection that requested shutdown has completed.
    fn wait_for_shutdown_connection(
        &self,
        connections: &mut Vec<Connection>,
    ) -> Result<(), DaemonError> {
        while self.lifecycle.is_shutdown_connection_active() {
            self.lifecycle
                .notifications()
                .recv()
                .map_err(|_| DaemonError::Thread)?;
            Connection::reap(connections)?;
        }

        Ok(())
    }

    /// Join one RPC listener thread.
    fn join_listener(listener: JoinHandle<Result<(), DaemonError>>) -> Result<(), DaemonError> {
        listener.join().map_err(|_| DaemonError::Thread)?
    }

    /// Return the root-bound workspace registry.
    pub(crate) const fn workspaces(&self) -> &WorkspaceRegistry {
        &self.workspaces
    }

    /// Return the runtime World registry.
    pub(crate) const fn worlds(&self) -> &WorldRegistry {
        &self.worlds
    }

    /// Return the shared process lifecycle.
    pub(crate) const fn lifecycle(&self) -> &DaemonLifecycle {
        &self.lifecycle
    }
}

/// One event from an RPC listener.
#[derive(Debug)]
enum Event {
    /// One accepted RPC transport.
    Accepted(Arc<dyn Transport>),
    /// One terminal listener failure.
    Failed(DaemonError),
}

impl Event {
    /// Receive one event from its listener.
    fn receive(
        event: Result<Self, crossbeam_channel::RecvError>,
    ) -> Result<Option<Self>, DaemonError> {
        let event = event.map_err(|_| DaemonError::Thread)?;

        Ok(Some(event))
    }

    /// Finish one event left by a stopped listener.
    fn finish(self) -> Result<(), DaemonError> {
        match self {
            Self::Accepted(transport) => transport.close().map_err(DaemonError::from),
            Self::Failed(error) => Err(error),
        }
    }
}

/// One live server-side RPC connection.
#[derive(Debug)]
struct Connection {
    /// Transport used to interrupt the connection during shutdown.
    transport: Arc<dyn Transport>,
    /// Server thread.
    thread: JoinHandle<Result<(), ServerError>>,
}

impl Connection {
    /// Spawn one server-side RPC connection.
    fn spawn(server: Server, transport: Arc<dyn Transport>, activity: ConnectionActivity) -> Self {
        let connection_transport = transport.clone();
        let thread = thread::spawn(move || {
            let result = server.serve(connection_transport);
            drop(activity);

            result
        });

        Self { transport, thread }
    }

    /// Join and remove completed connections.
    fn reap(connections: &mut Vec<Self>) -> Result<(), DaemonError> {
        let mut index = 0;

        // remove finished threads without shifting the remaining entries
        while index < connections.len() {
            if connections[index].thread.is_finished() {
                let connection = connections.swap_remove(index);
                connection.join()?;
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    /// Close this connection transport and interrupt pending input.
    fn close(&self) -> Result<(), TransportError> {
        self.transport.close()
    }

    /// Join this connection and propagate its failure.
    fn join(self) -> Result<(), DaemonError> {
        self.thread.join().map_err(|_| DaemonError::Thread)??;

        Ok(())
    }
}
