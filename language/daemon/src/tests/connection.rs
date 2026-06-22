use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use destack_artifact::DiskBlobStore;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Host, Repository, Settings,
};
use destack_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};
use destack_workspace::protocol::{
    FileOperationRequest, FrameCodec, OpenRootRequest, RootOpenOptions, WorkspaceRequest,
    WorkspaceResponse,
};
use destack_workspace::{
    Client, ClientOptions, FileOperation, Transport, TransportError, WorkspaceConnectOptions,
    WorkspaceConnection, WorkspaceEndpoint, WorkspaceIpcError, WorkspaceIpcListener,
};

use crate::daemon::{WorkspaceServer, WorkspaceServerOptions};

/// The workspace server result type used by connection tests.
#[cfg(unix)]
type WorkspaceServerResult = Result<(), crate::daemon::WorkspaceServerError>;

/// Unique suffix counter for ipc probe socket paths.
#[cfg(unix)]
static IPC_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Return true when connection tests may be skipped in constrained runtimes.
#[cfg(unix)]
fn allow_ipc_test_skip() -> bool {
    std::env::var_os("DESTACK_ALLOW_IPC_TEST_SKIP").is_some()
}

/// Validate that the runtime supports unix socket ipc for tests.
#[cfg(unix)]
fn ensure_ipc_test_environment() -> bool {
    let sequence = IPC_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    // probe unix domain socket bind support
    let probe_path = std::env::temp_dir().join(format!(
        "destack-ipc-probe-{}-{}.sock",
        std::process::id(),
        sequence,
    ));
    let _ = std::fs::remove_file(&probe_path);

    let bind_result = WorkspaceIpcListener::bind(&probe_path, 1024);
    let _ = std::fs::remove_file(&probe_path);

    // accept healthy probe results
    match bind_result {
        Ok(_) => true,
        Err(WorkspaceIpcError::Unsupported) => {
            if allow_ipc_test_skip() {
                return false;
            }

            panic!("ipc is unsupported in this runtime");
        }
        Err(WorkspaceIpcError::Io(error))
            if error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            if allow_ipc_test_skip() {
                return false;
            }

            panic!("ipc permission denied while binding unix socket: {error}")
        }
        Err(error) => panic!("unexpected ipc probe failure: {error}"),
    }
}

/// Build deterministic server options for connection tests.
#[cfg(unix)]
fn connection_test_server_options() -> WorkspaceServerOptions {
    // start from deterministic defaults
    let mut options = WorkspaceServerOptions {
        idle_shutdown: None,
        ..WorkspaceServerOptions::default()
    };

    // run provider work in one worker to avoid test contention
    options.worker_limit = 1;

    // return configured options
    options
}

/// Spawn a workspace server thread and return its completion channel.
#[cfg(unix)]
fn spawn_workspace_server(
    server: WorkspaceServer,
) -> (std::thread::JoinHandle<()>, Receiver<WorkspaceServerResult>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let result = server.serve();
        let _ = tx.send(result);
    });

    (handle, rx)
}

/// Join a workspace server thread and assert it exited cleanly.
#[cfg(unix)]
fn join_workspace_server(
    handle: std::thread::JoinHandle<()>,
    result_rx: Receiver<WorkspaceServerResult>,
) {
    let _ = handle.join();

    match result_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("workspace server exited with error: {error}"),
        Err(error) => panic!("missing workspace server result: {error}"),
    }
}

/// Harness for workspace server connection lifecycle tests.
#[cfg(unix)]
struct TestWorkspaceServer {
    /// Temporary root for this server.
    root: TemporaryPhysicalFileSystem,
    /// Shared repository backing server restarts.
    repository: Arc<Repository>,
    /// Stable workspace endpoint metadata.
    endpoint: WorkspaceEndpoint,
    /// Running server thread handle, when started.
    server_handle: Option<std::thread::JoinHandle<()>>,
    /// Running server result channel, when started.
    server_result_rx: Option<Receiver<WorkspaceServerResult>>,
}

#[cfg(unix)]
impl TestWorkspaceServer {
    /// Create a workspace server harness with an isolated root.
    fn new(prefix: &str) -> Self {
        // build a root and workspace endpoint
        let root = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let overrides = DestackLayoutOverride {
            home: Some(root.root().join("home")),
            ..DestackLayoutOverride::default()
        };
        let layout = DestackLayout::resolve(
            root.root(),
            root.root(),
            &environment,
            &Settings::default(),
            &overrides,
            None,
        );
        let host = Host::new(environment, file_system, Arc::new(DiskBlobStore::new()));
        let repository = Arc::new(Repository::new(
            root.root().to_path_buf(),
            host,
            Settings::default(),
            layout,
        ));
        let endpoint = WorkspaceEndpoint::new(repository.layout().home.clone());

        Self {
            root,
            repository,
            endpoint,
            server_handle: None,
            server_result_rx: None,
        }
    }

    /// Return the root path.
    fn root_path(&self) -> &std::path::Path {
        self.root.root()
    }

    /// Start the workspace server with deterministic test options.
    fn start(&mut self) {
        self.start_with_options(connection_test_server_options());
    }

    /// Start the workspace server with explicit options.
    fn start_with_options(&mut self, options: WorkspaceServerOptions) {
        // reject duplicate starts for a running server
        if self.server_handle.is_some() || self.server_result_rx.is_some() {
            panic!("workspace connection server is already running");
        }

        // spawn the workspace server thread
        let server =
            WorkspaceServer::with_options(self.repository.clone(), self.endpoint.clone(), options)
                .expect("workspace server should initialize");
        let (handle, server_result_rx) = spawn_workspace_server(server);
        self.server_handle = Some(handle);
        self.server_result_rx = Some(server_result_rx);
    }

    /// Connect to the running workspace server.
    fn connect(&self) -> WorkspaceConnection {
        let server_result_rx = self.server_result_rx.as_ref();
        wait_for_workspace_server(&self.endpoint, server_result_rx)
    }

    /// Send shutdown through a connection and join the server.
    fn shutdown_and_join(&mut self, connection: WorkspaceConnection) {
        let _ = connection.client.send_request(WorkspaceRequest::Shutdown);
        drop(connection);
        self.join();
    }

    /// Join a running server and validate a clean exit.
    fn join(&mut self) {
        let handle = self
            .server_handle
            .take()
            .expect("expected running workspace connection server handle");
        let result_rx = self
            .server_result_rx
            .take()
            .expect("expected running workspace connection server result channel");
        join_workspace_server(handle, result_rx);
    }

    /// Wait for server exit and join the thread when it finishes.
    fn wait_for_exit(
        &mut self,
        timeout: Duration,
    ) -> Result<WorkspaceServerResult, std::sync::mpsc::RecvTimeoutError> {
        let result_rx = self
            .server_result_rx
            .as_ref()
            .expect("expected running workspace connection server result channel");
        let result = result_rx.recv_timeout(timeout)?;

        let handle = self
            .server_handle
            .take()
            .expect("expected running workspace connection server handle");
        let _ = handle.join();
        let _ = self.server_result_rx.take();

        Ok(result)
    }
}

/// Reconnects after a client disconnect.
#[cfg(unix)]
#[test]
fn test_workspace_ipc_reconnect() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a workspace server harness
    let mut server = TestWorkspaceServer::new("workspace_ipc_reconnect");
    server.start();

    // connect to the workspace server and issue a ping
    let connection = server.connect();
    let response = connection.client.send_request(WorkspaceRequest::Ping);

    // assertion block
    assert!(
        matches!(response, Ok(WorkspaceResponse::Pong)),
        "{response:?}"
    );

    // drop the connection and reconnect
    drop(connection);
    let connection = server.connect();
    let response = connection.client.send_request(WorkspaceRequest::Ping);

    // assertion block
    assert!(
        matches!(response, Ok(WorkspaceResponse::Pong)),
        "{response:?}"
    );

    // request shutdown and join the server
    server.shutdown_and_join(connection);
}

/// Restarts cleanly after a shutdown.
#[cfg(unix)]
#[test]
fn test_workspace_ipc_restart() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a workspace server harness
    let mut server = TestWorkspaceServer::new("workspace_ipc_restart");
    server.start();

    // connect to the workspace server and issue a ping
    let connection = server.connect();
    let response = connection.client.send_request(WorkspaceRequest::Ping);

    // assertion block
    assert!(
        matches!(response, Ok(WorkspaceResponse::Pong)),
        "{response:?}"
    );

    // request shutdown and join the server
    server.shutdown_and_join(connection);

    // restart the workspace server
    server.start();

    // reconnect to the workspace server and issue another ping
    let connection = server.connect();
    let response = connection.client.send_request(WorkspaceRequest::Ping);

    // assertion block
    assert!(
        matches!(response, Ok(WorkspaceResponse::Pong)),
        "{response:?}"
    );

    // request shutdown and join the server
    server.shutdown_and_join(connection);
}

/// Opens a root again after restart and handles updates.
#[cfg(unix)]
#[test]
fn test_workspace_ipc_restart_resubscribe() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a workspace server harness
    let mut server = TestWorkspaceServer::new("workspace_ipc_resubscribe");
    server.start();

    // connect to the workspace server and open the root
    let connection = server.connect();
    let response = connection
        .client
        .send_request(WorkspaceRequest::OpenRoot(OpenRootRequest {
            workspace: server.root_path().to_path_buf(),
            root: server.root_path().to_path_buf(),
            options: RootOpenOptions::default(),
        }));
    let handle_id = match response {
        Ok(WorkspaceResponse::RootOpened(response)) => response.handle,
        other => panic!("unexpected response: {other:?}"),
    };

    // apply a file operation
    let file_path = server.root_path().join("main.ds");
    let operation = FileOperation::WriteText {
        path: file_path.clone(),
        content: "export const value = 1".to_string(),
    };
    let response = connection
        .client
        .send_request(WorkspaceRequest::ApplyFileOperation(FileOperationRequest {
            handle: handle_id,
            operation,
        }));
    match response {
        Ok(WorkspaceResponse::FileOperationApplied(response)) => {
            assert!(!response.updates.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    server.shutdown_and_join(connection);

    // restart the workspace server
    server.start();

    // reconnect and open the root again
    let connection = server.connect();
    let response = connection
        .client
        .send_request(WorkspaceRequest::OpenRoot(OpenRootRequest {
            workspace: server.root_path().to_path_buf(),
            root: server.root_path().to_path_buf(),
            options: RootOpenOptions::default(),
        }));
    let handle_id = match response {
        Ok(WorkspaceResponse::RootOpened(response)) => response.handle,
        other => panic!("unexpected response: {other:?}"),
    };

    // apply another file operation
    let operation = FileOperation::WriteText {
        path: file_path,
        content: "export const value = 2".to_string(),
    };
    let response = connection
        .client
        .send_request(WorkspaceRequest::ApplyFileOperation(FileOperationRequest {
            handle: handle_id,
            operation,
        }));
    match response {
        Ok(WorkspaceResponse::FileOperationApplied(response)) => {
            assert!(!response.updates.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    server.shutdown_and_join(connection);
}

/// Shuts down after an idle period with no connections.
#[cfg(unix)]
#[test]
fn test_workspace_ipc_idle_shutdown() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // configure the workspace server for a short idle shutdown
    let options = WorkspaceServerOptions {
        idle_shutdown: Some(Duration::from_millis(25)),
        ..WorkspaceServerOptions::default()
    };

    // start a workspace server harness
    let mut server = TestWorkspaceServer::new("workspace_ipc_idle_shutdown");
    server.start_with_options(options);

    // wait for the server to shut itself down
    let result = server
        .wait_for_exit(Duration::from_secs(2))
        .unwrap_or_else(|_| {
            let connection = server.connect();
            server.shutdown_and_join(connection);
            Ok(())
        });

    // assertion block
    assert!(result.is_ok());
}

/// Wait for workspace server metadata and connection to become available.
#[cfg(unix)]
fn wait_for_workspace_server(
    endpoint: &WorkspaceEndpoint,
    server_result_rx: Option<&Receiver<WorkspaceServerResult>>,
) -> WorkspaceConnection {
    // build connect options and deadline
    let options = WorkspaceConnectOptions {
        timeout: Duration::from_secs(1),
        retry_delay: Duration::from_millis(25),
        ..WorkspaceConnectOptions::default()
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    // poll for workspace metadata and connection
    while std::time::Instant::now() < deadline {
        // fail fast when the server thread exits before connect
        if let Some(server_result_rx) = server_result_rx {
            match server_result_rx.try_recv() {
                Ok(Ok(())) => panic!("workspace server exited before accepting connections"),
                Ok(Err(error)) => {
                    panic!("workspace server exited before accepting connections: {error}")
                }
                Err(TryRecvError::Disconnected) => {
                    panic!("workspace server thread disconnected before accepting connections");
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        if let Ok(Some(_)) = endpoint.read_metadata()
            && let Ok(connection) = endpoint.connect(options.clone(), None)
        {
            return connection;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // fail when the server never appears
    panic!(
        "failed to connect to workspace server: metadata_exists={} socket_exists={}",
        endpoint.read_metadata().ok().flatten().is_some(),
        endpoint.socket_path.exists()
    );
}

/// Round trip a workspace request over WebSocket.
#[cfg(unix)]
#[test]
fn test_workspace_websocket_roundtrip() {
    let mut server = TestWorkspaceServer::new("destack-workspace-websocket-ping");
    server.start();

    // wait for endpoint metadata and connect over WebSocket
    let metadata = wait_for_metadata(&server.endpoint, server.server_result_rx.as_ref());
    let transport = TestWebSocketTransport::connect(&metadata.websocket_url);
    let client = Client::new(Arc::new(transport));
    client
        .handshake(ClientOptions::default())
        .expect("websocket handshake should complete");

    // send a protocol ping over the WebSocket connection
    let response = client
        .send_request(WorkspaceRequest::Ping)
        .expect("websocket ping should succeed");
    assert!(matches!(response, WorkspaceResponse::Pong));

    // shut down and join the server over the same transport
    let response = client
        .send_request(WorkspaceRequest::Shutdown)
        .expect("websocket shutdown should succeed");
    assert!(matches!(response, WorkspaceResponse::ShutdownAck));
    server.join();
}

/// Wait for workspace server metadata to become available.
#[cfg(unix)]
fn wait_for_metadata(
    endpoint: &WorkspaceEndpoint,
    server_result_rx: Option<&Receiver<WorkspaceServerResult>>,
) -> destack_workspace::WorkspaceServerMetadata {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    while std::time::Instant::now() < deadline {
        // fail fast when the server thread exits before publishing metadata
        if let Some(server_result_rx) = server_result_rx {
            match server_result_rx.try_recv() {
                Ok(Ok(())) => panic!("workspace server exited before publishing metadata"),
                Ok(Err(error)) => {
                    panic!("workspace server exited before publishing metadata: {error}")
                }
                Err(TryRecvError::Disconnected) => {
                    panic!("workspace server thread disconnected before publishing metadata");
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        if let Ok(Some(metadata)) = endpoint.read_metadata() {
            return metadata;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    panic!("failed to read workspace server metadata");
}

/// Test WebSocket transport for workspace protocol tests.
#[cfg(unix)]
struct TestWebSocketTransport {
    /// TCP stream connected to the workspace server.
    stream: parking_lot::Mutex<std::net::TcpStream>,
    /// Protocol frame codec.
    codec: FrameCodec,
}

#[cfg(unix)]
impl TestWebSocketTransport {
    /// Connect to a WebSocket URL.
    fn connect(url: &str) -> Self {
        let endpoint = TestWebSocketEndpoint::parse(url);
        let mut stream = std::net::TcpStream::connect(&endpoint.address)
            .expect("websocket tcp connection should open");

        // perform the WebSocket upgrade
        let request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: dGVzdGluZy1kZXN0YWNrIQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n",
            endpoint.path, endpoint.address
        );
        std::io::Write::write_all(&mut stream, request.as_bytes())
            .expect("websocket request should write");
        read_websocket_handshake_response(&mut stream);

        Self {
            stream: parking_lot::Mutex::new(stream),
            codec: FrameCodec::default(),
        }
    }
}

#[cfg(unix)]
impl Transport for TestWebSocketTransport {
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        let mut stream = self.stream.lock();
        let frame = self.codec.encode(payload)?;
        write_masked_websocket_message(&mut stream, &frame)?;

        Ok(())
    }

    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        let mut stream = self.stream.lock();
        let frame = read_unmasked_websocket_message(&mut stream)?;

        self.codec.decode(&frame).map_err(TransportError::from)
    }

    fn close(&self) {}
}

/// Parsed WebSocket endpoint used by tests.
#[cfg(unix)]
struct TestWebSocketEndpoint {
    /// Socket address.
    address: String,
    /// Request path and query.
    path: String,
}

#[cfg(unix)]
impl TestWebSocketEndpoint {
    /// Parse one local WebSocket URL.
    fn parse(url: &str) -> Self {
        let url = url
            .strip_prefix("ws://")
            .expect("test websocket url should use ws");
        let slash = url.find('/').expect("test websocket url should have path");
        let address = url[..slash].to_string();
        let path = url[slash..].to_string();

        Self { address, path }
    }
}

/// Read one WebSocket upgrade response.
#[cfg(unix)]
fn read_websocket_handshake_response(stream: &mut std::net::TcpStream) {
    let mut response = Vec::new();
    let mut buffer = [0u8; 512];

    loop {
        let len = std::io::Read::read(stream, &mut buffer)
            .expect("websocket response should be readable");
        assert_ne!(len, 0, "websocket response ended early");
        response.extend_from_slice(&buffer[..len]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let response = String::from_utf8(response).expect("websocket response should be utf8");
    assert!(
        response.starts_with("HTTP/1.1 101 "),
        "expected websocket upgrade, got {response:?}"
    );
}

/// Write one masked client WebSocket binary message.
#[cfg(unix)]
fn write_masked_websocket_message(
    stream: &mut std::net::TcpStream,
    payload: &[u8],
) -> Result<(), TransportError> {
    let mask = [1u8, 2, 3, 4];
    let mut header = Vec::new();
    header.push(0x82);
    if payload.len() <= 125 {
        header.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(0x80 | 126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    header.extend_from_slice(&mask);

    // mask payload bytes as required for client frames
    let payload = payload
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ mask[index % 4])
        .collect::<Vec<_>>();

    std::io::Write::write_all(stream, &header)?;
    std::io::Write::write_all(stream, &payload)?;
    std::io::Write::flush(stream)?;

    Ok(())
}

/// Read one unmasked server WebSocket binary message.
#[cfg(unix)]
fn read_unmasked_websocket_message(
    stream: &mut std::net::TcpStream,
) -> Result<Vec<u8>, TransportError> {
    let mut header = [0u8; 2];
    std::io::Read::read_exact(stream, &mut header)?;
    let opcode = header[0] & 0x0f;
    assert_eq!(opcode, 0x2, "expected binary websocket frame");
    assert_eq!(header[1] & 0x80, 0, "server frame should be unmasked");

    let mut len = usize::from(header[1] & 0x7f);
    if len == 126 {
        let mut bytes = [0u8; 2];
        std::io::Read::read_exact(stream, &mut bytes)?;
        len = usize::from(u16::from_be_bytes(bytes));
    } else if len == 127 {
        let mut bytes = [0u8; 8];
        std::io::Read::read_exact(stream, &mut bytes)?;
        len = usize::try_from(u64::from_be_bytes(bytes)).expect("test payload should fit usize");
    }

    let mut payload = vec![0u8; len];
    std::io::Read::read_exact(stream, &mut payload)?;

    Ok(payload)
}
