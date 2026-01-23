use std::sync::Arc;
use std::time::Duration;

use destack_source::TemporaryPhysicalFileSystem;
use destack_workspace::Session;

use crate::daemon::{DaemonInstance, DaemonServer, DaemonServerOptions, DaemonShutdownOptions};
use crate::protocol::{
    DaemonRequest, DaemonResponse, FileUpdate, FileUpdateKind, FileUpdateRequest,
    OpenWorkspaceRequest, WorkspaceOpenOptions,
};
use crate::{DaemonConnectOptions, connect_ipc_daemon};

/// Round trip a daemon request over ipc.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_roundtrip() {
    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_roundtrip");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let server = DaemonServer::new(session, instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    let _ = handle.join();
}

/// Reconnects after a client disconnect.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_reconnect() {
    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_reconnect");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let server = DaemonServer::new(session, instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // drop the connection and reconnect
    drop(connection);
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    let _ = handle.join();
}

/// Restarts cleanly after a shutdown.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart() {
    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_restart");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let server = DaemonServer::new(session.clone(), instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // connect to the daemon and issue a ping
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    let _ = handle.join();

    // restart the daemon server
    let server = DaemonServer::new(session, instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // reconnect to the daemon and issue another ping
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)), "{response:?}");

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    let _ = handle.join();
}

/// Opens a workspace again after restart and handles updates.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_restart_resubscribe() {
    // build a workspace and daemon instance
    let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_ipc_resubscribe");
    let session = Arc::new(Session::new(root.root().to_path_buf()));
    let cache_root = session.workspace_cache_dir();
    let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

    // start the daemon server
    let server = DaemonServer::new(session.clone(), instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // connect to the daemon and open the workspace
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::OpenWorkspace(
        OpenWorkspaceRequest {
            root: root.root().to_path_buf(),
            options: WorkspaceOpenOptions::default(),
        },
    ));
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
    let response = connection.client.send_request(DaemonRequest::ApplyFileUpdate(
        FileUpdateRequest {
            handle: handle_id,
            update,
        },
    ));
    match response {
        Ok(DaemonResponse::FileUpdated(response)) => {
            assert!(!response.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    drop(connection);
    let _ = handle.join();

    // restart the daemon server
    let server = DaemonServer::new(session, instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // reconnect and open the workspace again
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::OpenWorkspace(
        OpenWorkspaceRequest {
            root: root.root().to_path_buf(),
            options: WorkspaceOpenOptions::default(),
        },
    ));
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
    let response = connection.client.send_request(DaemonRequest::ApplyFileUpdate(
        FileUpdateRequest {
            handle: handle_id,
            update,
        },
    ));
    match response {
        Ok(DaemonResponse::FileUpdated(response)) => {
            assert!(!response.updates.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // request shutdown and join the server
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    let _ = handle.join();
}

/// Shuts down after an idle period with no connections.
#[cfg(unix)]
#[test]
fn test_daemon_ipc_idle_shutdown() {
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
        let connection = wait_for_daemon(&instance);
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
fn wait_for_daemon(instance: &DaemonInstance) -> crate::daemon::DaemonConnection {
    // build connect options and deadline
    let options = DaemonConnectOptions {
        timeout: Duration::from_secs(1),
        retry_delay: Duration::from_millis(25),
        ..DaemonConnectOptions::default()
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    // poll for the daemon metadata and connection
    while std::time::Instant::now() < deadline {
        if let Ok(Some(_)) = instance.read_metadata()
            && let Ok(connection) = connect_ipc_daemon(instance, options.clone(), None)
        {
            return connection;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // fail when the daemon never appears
    panic!("failed to connect to daemon");
}
