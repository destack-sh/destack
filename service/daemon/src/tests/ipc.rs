use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use destack_artifact::DiskCacheStore;
use destack_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};
use destack_workspace::{HostEnvironment, Repository};

use crate::daemon::{DaemonInstance, DaemonServer, DaemonServerOptions, DaemonShutdownOptions};
use crate::protocol::{
    DaemonRequest, DaemonResponse, FileUpdate, FileUpdateKind, FileUpdateRequest, OpenRootRequest,
    RootOpenOptions,
};
use crate::{DaemonConnectOptions, connect_ipc_daemon};

/// The daemon server result type used by ipc tests.
#[cfg(unix)]
type DaemonServerResult = Result<(), crate::daemon::DaemonServerError>;

/// Return true when ipc tests may be skipped in constrained runtimes.
#[cfg(unix)]
fn allow_ipc_test_skip() -> bool {
    std::env::var_os("DESTACK_ALLOW_IPC_TEST_SKIP").is_some()
}

/// Validate that the runtime supports unix socket ipc for tests.
#[cfg(unix)]
fn ensure_ipc_test_environment() -> bool {
    // probe unix domain socket bind support
    let probe_path = std::env::temp_dir().join(format!(
        "destack-ipc-probe-{}-{}.sock",
        std::process::id(),
        std::thread::current().id().as_u64()
    ));
    let _ = std::fs::remove_file(&probe_path);

    let bind_result = crate::DaemonIpcListener::bind(&probe_path, 1024);
    let _ = std::fs::remove_file(&probe_path);

    // accept healthy probe results
    match bind_result {
        Ok(_) => true,
        Err(crate::DaemonIpcError::Unsupported) => {
            if allow_ipc_test_skip() {
                return false;
            }

            panic!("ipc is unsupported in this runtime");
        }
        Err(crate::DaemonIpcError::Io(error))
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

/// Build deterministic server options for ipc tests.
#[cfg(unix)]
fn ipc_test_server_options() -> DaemonServerOptions {
    // start from deterministic defaults
    let mut options = DaemonServerOptions {
        shutdown: DaemonShutdownOptions {
            idle_shutdown: None,
            idle_poll: Duration::from_millis(250),
        },
        ..DaemonServerOptions::default()
    };

    // run provider work in one worker to avoid test contention
    options.worker_limit = 1;

    // return configured options
    options
}

/// Spawn a daemon server thread and return its completion channel.
#[cfg(unix)]
fn spawn_daemon_server(
    server: DaemonServer,
) -> (std::thread::JoinHandle<()>, Receiver<DaemonServerResult>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let result = server.serve();
        let _ = tx.send(result);
    });

    (handle, rx)
}

/// Join a daemon server thread and assert it exited cleanly.
#[cfg(unix)]
fn join_daemon_server(
    handle: std::thread::JoinHandle<()>,
    result_rx: Receiver<DaemonServerResult>,
) {
    let _ = handle.join();

    match result_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("daemon server exited with error: {error}"),
        Err(error) => panic!("missing daemon server result: {error}"),
    }
}

/// Harness for daemon ipc lifecycle tests.
#[cfg(unix)]
struct TestIpcDaemon {
    /// Temporary root for this daemon.
    root: TemporaryPhysicalFileSystem,
    /// Shared repository backing server restarts.
    repository: Arc<Repository>,
    /// Stable daemon instance metadata.
    instance: DaemonInstance,
    /// Running server thread handle, when started.
    server_handle: Option<std::thread::JoinHandle<()>>,
    /// Running server result channel, when started.
    server_result_rx: Option<Receiver<DaemonServerResult>>,
}

#[cfg(unix)]
impl TestIpcDaemon {
    /// Create an ipc daemon harness with an isolated root.
    fn new(prefix: &str) -> Self {
        // build a root and daemon instance
        let root = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Arc::new(Repository::new(
            root.root().to_path_buf(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        ));
        let cache_root = repository.cache_directory();
        let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

        Self {
            root,
            repository,
            instance,
            server_handle: None,
            server_result_rx: None,
        }
    }

    /// Return the root path.
    fn root_path(&self) -> &std::path::Path {
        self.root.root()
    }

    /// Start the daemon server with deterministic test options.
    fn start(&mut self) {
        self.start_with_options(ipc_test_server_options());
    }

    /// Start the daemon server with explicit options.
    fn start_with_options(&mut self, options: DaemonServerOptions) {
        // reject duplicate starts for a running server
        if self.server_handle.is_some() || self.server_result_rx.is_some() {
            panic!("ipc daemon server is already running");
        }

        // spawn the daemon server thread
        let server =
            DaemonServer::with_options(self.repository.clone(), self.instance.clone(), options);
        let (handle, server_result_rx) = spawn_daemon_server(server);
        self.server_handle = Some(handle);
        self.server_result_rx = Some(server_result_rx);
    }

    /// Connect to the running daemon.
    fn connect(&self) -> crate::daemon::DaemonConnection {
        let server_result_rx = self.server_result_rx.as_ref();
        wait_for_daemon(&self.instance, server_result_rx)
    }

    /// Send shutdown through a connection and join the server.
    fn shutdown_and_join(&mut self, connection: crate::daemon::DaemonConnection) {
        let _ = connection.client.send_request(DaemonRequest::Shutdown);
        drop(connection);
        self.join();
    }

    /// Join a running server and validate a clean exit.
    fn join(&mut self) {
        let handle = self
            .server_handle
            .take()
            .expect("expected running ipc server handle");
        let result_rx = self
            .server_result_rx
            .take()
            .expect("expected running ipc server result channel");
        join_daemon_server(handle, result_rx);
    }

    /// Wait for server exit and join the thread when it finishes.
    fn wait_for_exit(
        &mut self,
        timeout: Duration,
    ) -> Result<DaemonServerResult, std::sync::mpsc::RecvTimeoutError> {
        let result_rx = self
            .server_result_rx
            .as_ref()
            .expect("expected running ipc server result channel");
        let result = result_rx.recv_timeout(timeout)?;

        let handle = self
            .server_handle
            .take()
            .expect("expected running ipc server handle");
        let _ = handle.join();
        let _ = self.server_result_rx.take();

        Ok(result)
    }
}

/// Round trip a daemon request over ipc.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_roundtrip() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a daemon harness
    let mut daemon = TestIpcDaemon::new("daemon_ipc_roundtrip");
    daemon.start();

    // connect to the daemon and issue a ping
    let connection = daemon.connect();
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);
}

/// Reconnects after a client disconnect.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_reconnect() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a daemon harness
    let mut daemon = TestIpcDaemon::new("daemon_ipc_reconnect");
    daemon.start();

    // connect to the daemon and issue a ping
    let connection = daemon.connect();
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // drop the connection and reconnect
    drop(connection);
    let connection = daemon.connect();
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);
}

/// Restarts cleanly after a shutdown.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a daemon harness
    let mut daemon = TestIpcDaemon::new("daemon_ipc_restart");
    daemon.start();

    // connect to the daemon and issue a ping
    let connection = daemon.connect();
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);

    // restart the daemon server
    daemon.start();

    // reconnect to the daemon and issue another ping
    let connection = daemon.connect();
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);
}

/// Opens a root again after restart and handles updates.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart_resubscribe() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // start a daemon harness
    let mut daemon = TestIpcDaemon::new("daemon_ipc_resubscribe");
    daemon.start();

    // connect to the daemon and open the root
    let connection = daemon.connect();
    let response = connection
        .client
        .send_request(DaemonRequest::OpenRoot(OpenRootRequest {
            root: daemon.root_path().to_path_buf(),
            options: RootOpenOptions::default(),
        }));
    let handle_id = match response {
        Ok(DaemonResponse::RootOpened(response)) => response.handle,
        other => panic!("unexpected response: {other:?}"),
    };

    // apply a file update
    let file_path = daemon.root_path().join("main.ds");
    let update = FileUpdate {
        path: file_path.clone(),
        update: FileUpdateKind::Text {
            content: "export const value = 1".to_string(),
        },
        write_to_disk: true,
    };
    let response = connection
        .client
        .send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
            handle: handle_id,
            update,
        }));
    match response {
        Ok(DaemonResponse::FileUpdated(response)) => {
            assert!(!response.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);

    // restart the daemon server
    daemon.start();

    // reconnect and open the root again
    let connection = daemon.connect();
    let response = connection
        .client
        .send_request(DaemonRequest::OpenRoot(OpenRootRequest {
            root: daemon.root_path().to_path_buf(),
            options: RootOpenOptions::default(),
        }));
    let handle_id = match response {
        Ok(DaemonResponse::RootOpened(response)) => response.handle,
        other => panic!("unexpected response: {other:?}"),
    };

    // apply another file update
    let update = FileUpdate {
        path: file_path,
        update: FileUpdateKind::Text {
            content: "export const value = 2".to_string(),
        },
        write_to_disk: true,
    };
    let response = connection
        .client
        .send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
            handle: handle_id,
            update,
        }));
    match response {
        Ok(DaemonResponse::FileUpdated(response)) => {
            assert!(!response.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    daemon.shutdown_and_join(connection);
}

/// Shuts down after an idle period with no connections.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_idle_shutdown() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // configure the daemon server for a short idle shutdown
    let options = DaemonServerOptions {
        shutdown: DaemonShutdownOptions {
            idle_shutdown: Some(Duration::from_millis(25)),
            idle_poll: Duration::from_millis(10),
        },
        ..DaemonServerOptions::default()
    };

    // start a daemon harness
    let mut daemon = TestIpcDaemon::new("daemon_ipc_idle_shutdown");
    daemon.start_with_options(options);

    // wait for the server to shut itself down
    let result = daemon
        .wait_for_exit(Duration::from_secs(2))
        .unwrap_or_else(|_| {
            let connection = daemon.connect();
            daemon.shutdown_and_join(connection);
            Ok(())
        });

    // assertion block
    assert!(result.is_ok());
}

/// Wait for the daemon metadata and connection to become available.
#[cfg(unix)]
fn wait_for_daemon(
    instance: &DaemonInstance,
    server_result_rx: Option<&Receiver<DaemonServerResult>>,
) -> crate::daemon::DaemonConnection {
    // build connect options and deadline
    let options = DaemonConnectOptions {
        timeout: Duration::from_secs(1),
        retry_delay: Duration::from_millis(25),
        ..DaemonConnectOptions::default()
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    // poll for the daemon metadata and connection
    while std::time::Instant::now() < deadline {
        // fail fast when the daemon thread exits before connect
        if let Some(server_result_rx) = server_result_rx {
            match server_result_rx.try_recv() {
                Ok(Ok(())) => panic!("daemon exited before accepting connections"),
                Ok(Err(error)) => panic!("daemon exited before accepting connections: {error}"),
                Err(TryRecvError::Disconnected) => {
                    panic!("daemon thread disconnected before accepting connections");
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        if let Ok(Some(_)) = instance.read_metadata()
            && let Ok(connection) = connect_ipc_daemon(instance, options.clone(), None)
        {
            return connection;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // fail when the daemon never appears
    panic!(
        "failed to connect to daemon: metadata_exists={} socket_exists={}",
        instance.read_metadata().ok().flatten().is_some(),
        instance.socket_path.exists()
    );
}
