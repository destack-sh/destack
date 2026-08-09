use std::sync::Arc;
use std::thread::JoinHandle;

use crate::{
    Connection, ConnectionOptions, LoopbackTransport, Registry, Server, ServerError, Service,
};

/// One live loopback server and its negotiated client connection.
pub(super) struct TestServer {
    /// Client connection retained until explicit shutdown.
    connection: Option<Arc<Connection>>,
    /// Blocking server connection task.
    server: Option<JoinHandle<Result<(), ServerError>>>,
}

impl TestServer {
    /// Start one service with default connection options.
    pub(super) fn new(service: impl Service) -> Self {
        Self::with_options(
            service,
            ConnectionOptions::new("test-server"),
            ConnectionOptions::new("test-client"),
        )
    }

    /// Start one service with exact connection options.
    pub(super) fn with_options(
        service: impl Service,
        server_options: ConnectionOptions,
        client_options: ConnectionOptions,
    ) -> Self {
        let service = service;
        let service_id = service.schema().id();
        let (client_transport, server_transport) = LoopbackTransport::pair(64);
        let mut registry = Registry::new();
        registry.insert(service).expect("register test service");
        let server = Server::new(registry, server_options);
        let server = std::thread::spawn(move || server.serve(Arc::new(server_transport)));
        let connection =
            Connection::connect(Arc::new(client_transport), client_options, vec![service_id])
                .expect("connect test client");

        Self {
            connection: Some(connection),
            server: Some(server),
        }
    }

    /// Return the negotiated client connection.
    pub(super) fn connection(&self) -> &Arc<Connection> {
        self.connection.as_ref().expect("test connection is open")
    }

    /// Close the client connection and join the server.
    pub(super) fn close(mut self) {
        let connection = self.connection.take().expect("test connection is open");
        connection.close().expect("close test connection");
        drop(connection);

        self.join().expect("serve test connection");
    }

    /// Drop the client connection and join the server.
    pub(super) fn disconnect(mut self) {
        drop(self.connection.take());

        self.join().expect("observe test connection closure");
    }

    /// Join the blocking server task.
    pub(super) fn join(&mut self) -> Result<(), ServerError> {
        self.server
            .take()
            .expect("test server is running")
            .join()
            .expect("join test server")
    }
}
