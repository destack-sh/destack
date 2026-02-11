use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use destack_source::TemporaryPhysicalFileSystem;
use destack_workspace::Session;

use crate::daemon::{DaemonInstance, DaemonServer, DaemonServerOptions, DaemonShutdownOptions};
use crate::protocol::{
    DaemonRequest, DaemonResponse, FileUpdate, FileUpdateKind, FileUpdateRequest,
    OpenWorkspaceRequest, WorkspaceOpenOptions,
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
    DaemonServerOptions {
        shutdown: DaemonShutdownOptions {
            idle_shutdown: None,
            idle_poll: Duration::from_millis(250),
        },
        ..DaemonServerOptions::default()
    }
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

/// Round trip a daemon request over ipc.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_roundtrip() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_roundtrip");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session, instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    join_daemon_server(handle, server_result_rx);
}

/// Reconnects after a client disconnect.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_reconnect() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_reconnect");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session, instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // drop the connection and reconnect
    drop(connection);
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    join_daemon_server(handle, server_result_rx);
}

/// Restarts cleanly after a shutdown.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_restart");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session.clone(), instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    join_daemon_server(handle, server_result_rx);

    // restart the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session, instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // reconnect to the daemon and issue another ping
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    join_daemon_server(handle, server_result_rx);
}

/// Opens a workspace again after restart and handles updates.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart_resubscribe() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_resubscribe");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session.clone(), instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // connect to the daemon and open the workspace
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response =
        connection
            .client
            .send_request(DaemonRequest::OpenWorkspace(OpenWorkspaceRequest {
                root: root.root().to_path_buf(),
                options: WorkspaceOpenOptions::default(),
            }));
    let handle_id = match response {
        Ok(DaemonResponse::WorkspaceOpened(response)) => response.handle,
        other => panic!("unexpected response: {other:?}"),
    };

    // apply a file update
    let file_path = root.root().join("main.ds");
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
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    drop(connection);
    join_daemon_server(handle, server_result_rx);

    // restart the daemon server
    let options = ipc_test_server_options();
    let server = DaemonServer::with_options(session, instance.clone(), options);
    let (handle, server_result_rx) = spawn_daemon_server(server);

    // reconnect and open the workspace again
    let connection = wait_for_daemon(&instance, Some(&server_result_rx));
    let response =
        connection
            .client
            .send_request(DaemonRequest::OpenWorkspace(OpenWorkspaceRequest {
                root: root.root().to_path_buf(),
                options: WorkspaceOpenOptions::default(),
            }));
    let handle_id = match response {
        Ok(DaemonResponse::WorkspaceOpened(response)) => response.handle,
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
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    join_daemon_server(handle, server_result_rx);
}

/// Shuts down after an idle period with no connections.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_idle_shutdown() {
    if !ensure_ipc_test_environment() {
        return;
    }

    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_idle_shutdown");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // configure the daemon server for a short idle shutdown
    let options = DaemonServerOptions {
        shutdown: DaemonShutdownOptions {
            idle_shutdown: Some(Duration::from_millis(25)),
            idle_poll: Duration::from_millis(10),
        },
        ..DaemonServerOptions::default()
    };

    // start the daemon server
    let server = DaemonServer::with_options(session, instance.clone(), options);
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let _ = tx.send(server.serve());
    });

    // wait for the server to shut itself down
    let result = rx.recv_timeout(Duration::from_secs(2)).unwrap_or_else(|_| {
        let connection = wait_for_daemon(&instance, None);
        let _ = connection.client.send_request(DaemonRequest::Shutdown);
        rx.recv_timeout(Duration::from_secs(2))
            .expect("daemon shutdown timeout")
    });

    // assertion block
    assert!(result.is_ok());
    let _ = handle.join();
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
