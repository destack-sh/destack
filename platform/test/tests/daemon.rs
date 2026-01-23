use std::sync::Arc;
use std::time::Duration;

use destack_cli::common::ProgramArgs;
use destack_cli::pipeline::daemon::run_daemon_command_with_session;
use destack_daemon::protocol::{DaemonRequest, DaemonResponse};
use destack_daemon::{
    CommandCheckOptions, CommandInput, CommandPayload, CommonCommandOptions, DaemonConnectOptions,
    DaemonInstance, DaemonServer, connect_ipc_daemon,
};
use destack_lsp::tests::harness::{LspHarness, LspHarnessMode, uri_for_path};
use destack_source::{DiagnosticOptions, PhysicalFileSystem, TemporaryPhysicalFileSystem};
use destack_workspace::{MemoryCacheStore, Session};

/// Ensure CLI, daemon IPC, and LSP wire together.
#[tokio::test]
async fn test_cli_daemon_lsp_ipc() {
    // build a physical workspace fixture
    let fs = TemporaryPhysicalFileSystem::new_with_prefix("daemon_stack");
    let root = fs.root().to_path_buf();
    let _ = fs.write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n");
    let main_path = fs
        .write_text("main.ds", "export const x: number = 1;\n")
        .expect("main.ds should be written");

    // build a session for the daemon server
    let session = Arc::new(
        Session::new(root.clone())
            .with_fs(Arc::new(PhysicalFileSystem))
            .with_cache_store(Arc::new(MemoryCacheStore::new())),
    );

    // start a daemon server in the background
    let instance = DaemonInstance::from_session(&session);
    let server = DaemonServer::new(session.clone(), instance.clone());
    let handle = std::thread::spawn(move || server.serve());

    // wait for the daemon to accept ipc connections
    let connection = wait_for_daemon(&instance);
    let response = connection.client.send_request(DaemonRequest::Ping);

    // assertion block
    assert!(matches!(response, Ok(DaemonResponse::Pong)));

    // run a check command through the cli daemon pipeline
    let program = ProgramArgs {
        cwd: Some(root.clone()),
        workspace: Some(root.clone()),
        ..ProgramArgs::default()
    };
    let common = CommonCommandOptions {
        inputs: vec![CommandInput::File {
            path: main_path.clone(),
        }],
        ..Default::default()
    };
    let payload = CommandPayload::Check(CommandCheckOptions::default());
    let result = run_daemon_command_with_session(
        session.clone(),
        &program,
        DiagnosticOptions::default(),
        common,
        payload,
        None,
    )
    .expect("daemon check should succeed");

    // assertion block
    assert_eq!(result.response.exit_code, 0);
    assert!(result.diagnostics.is_empty());

    // initialize a daemon backed lsp service
    let mut harness = LspHarness::new_with_mode(root.clone(), LspHarnessMode::Ipc);
    harness.initialize().await;

    // open a document and confirm diagnostics flow
    let uri = uri_for_path(&main_path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;
    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block
    assert_eq!(diagnostics.uri, uri);
    assert!(diagnostics.diagnostics.is_empty());

    // introduce a syntax error and expect diagnostics
    harness
        .did_change(uri.clone(), "export const x = ;\n", 2)
        .await;
    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());

    // fix the document and expect diagnostics to clear
    harness
        .did_change(uri.clone(), "export const x: number = 1;\n", 3)
        .await;
    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // assertion block
    assert_eq!(diagnostics.uri, uri);
    assert!(diagnostics.diagnostics.is_empty());

    // shut down the lsp connection before stopping the daemon
    harness.shutdown().await;
    drop(harness);

    // request daemon shutdown
    let _ = connection.client.send_request(DaemonRequest::Shutdown);
    drop(connection);
    let _ = handle.join();
}

/// Wait for the daemon to accept ipc connections.
fn wait_for_daemon(instance: &DaemonInstance) -> destack_daemon::DaemonConnection {
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
