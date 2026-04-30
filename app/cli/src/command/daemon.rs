use std::path::PathBuf;

use clap::{Args, Subcommand};

use destack_daemon::protocol::DaemonRequest;
use destack_daemon::{
    DaemonConnectOptions, DaemonInstance, DaemonServer, DaemonServerOptions, connect_ipc_daemon,
};

use crate::common::diagnostic::DiagnosticArgs;
use crate::common::program::ProgramArgs;
use crate::console;
use crate::pipeline::daemon::DaemonLaunchContext;

/// Arguments for the daemon command.
#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

/// Daemon command variants.
#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Run the daemon server in the foreground.
    Serve(DaemonServeArgs),
    /// Start the daemon for the workspace.
    Start(DaemonLifecycleArgs),
    /// Stop the daemon for the workspace.
    Stop(DaemonLifecycleArgs),
    /// Show daemon status for the workspace.
    Status(DaemonLifecycleArgs),
}

/// Arguments for running the daemon server.
#[derive(Args, Debug)]
pub struct DaemonServeArgs {
    #[command(flatten)]
    pub program: ProgramArgs,

    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Override the daemon socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

/// Arguments for daemon lifecycle commands.
#[derive(Args, Debug)]
pub struct DaemonLifecycleArgs {
    #[command(flatten)]
    pub program: ProgramArgs,
}

/// Run the daemon command.
pub fn run(args: &DaemonArgs) -> i32 {
    // dispatch to the selected subcommand
    match &args.command {
        DaemonCommand::Serve(args) => run_serve(args),
        DaemonCommand::Start(args) => run_start(args),
        DaemonCommand::Stop(args) => run_stop(args),
        DaemonCommand::Status(args) => run_status(args),
    }
}

/// Run the daemon server in the foreground.
fn run_serve(args: &DaemonServeArgs) -> i32 {
    // build a repository
    let repository = args.program.setup();

    // resolve daemon instance metadata
    let mut instance = DaemonInstance::from_repository(&repository);

    // override the socket path when requested
    if let Some(socket) = args.socket.as_ref() {
        instance.socket_path = socket.clone();
    }

    // build server options from the repository
    let mut server_options = DaemonServerOptions::from_repository(&repository);
    server_options.worker_limit = args.program.workers as usize;
    let server = DaemonServer::with_options(repository, instance, server_options);

    // serve until shutdown
    if let Err(error) = server.serve() {
        console::error(&format!("daemon error: {error}"));
        return 1;
    }

    0
}

/// Start the daemon for the workspace.
fn run_start(args: &DaemonLifecycleArgs) -> i32 {
    // build repository metadata
    let repository = args.program.setup();
    let instance = DaemonInstance::from_repository(&repository);
    let launch_context = DaemonLaunchContext::from_program(&args.program);
    let launch = launch_context.build_launch_config(&instance);
    let options = DaemonConnectOptions::default();
    let result = connect_ipc_daemon(&instance, options, Some(launch));

    // report connectivity status
    match result {
        Ok(_) => {
            console::info("daemon is running");
            0
        }
        Err(error) => {
            console::error(&format!("failed to start daemon: {error}"));
            1
        }
    }
}

/// Stop the daemon for the workspace.
fn run_stop(args: &DaemonLifecycleArgs) -> i32 {
    // build repository metadata
    let repository = args.program.setup();
    let instance = DaemonInstance::from_repository(&repository);
    let options = DaemonConnectOptions::default();
    let connection = match connect_ipc_daemon(&instance, options, None) {
        Ok(connection) => connection,
        Err(error) => {
            console::warn(&format!("daemon not running: {error}"));
            return 1;
        }
    };

    // request shutdown and report the result
    let response = connection.client.send_request(DaemonRequest::Shutdown);
    match response {
        Ok(_) => {
            console::info("daemon shutdown requested");
            0
        }
        Err(error) => {
            console::error(&format!("daemon shutdown failed: {error}"));
            1
        }
    }
}

/// Report daemon status for the workspace.
fn run_status(args: &DaemonLifecycleArgs) -> i32 {
    // build repository metadata
    let repository = args.program.setup();
    let instance = DaemonInstance::from_repository(&repository);
    let options = DaemonConnectOptions::default();

    // probe daemon connectivity
    match connect_ipc_daemon(&instance, options, None) {
        Ok(_) => {
            console::info("daemon is running");
            0
        }
        Err(error) => {
            // fall back to cached metadata when available
            if let Ok(Some(metadata)) = instance.read_metadata() {
                console::warn(&format!(
                    "daemon not responding (pid {}, started at {})",
                    metadata.pid, metadata.started_at
                ));
            } else {
                console::warn(&format!("daemon not running: {error}"));
            }
            1
        }
    }
}
