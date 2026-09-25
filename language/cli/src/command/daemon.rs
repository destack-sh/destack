use std::path::PathBuf;

use clap::{Args, Subcommand};

use tspp_daemon::{Daemon, DaemonConnectOptions, DaemonEndpoint, DaemonOptions};

use crate::common::program::ProgramArgs;
use crate::console;

/// Arguments for the daemon command.
#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

/// Daemon command variants.
#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Run the daemon in the foreground.
    Serve(DaemonServeArgs),
    /// Start the daemon.
    Start(DaemonLifecycleArgs),
    /// Stop the daemon.
    Stop(DaemonLifecycleArgs),
    /// Show daemon status.
    Status(DaemonLifecycleArgs),
}

/// Arguments for running the daemon.
#[derive(Args, Debug)]
pub struct DaemonServeArgs {
    #[command(flatten)]
    pub program: ProgramArgs,

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

/// Run the daemon in the foreground.
fn run_serve(args: &DaemonServeArgs) -> i32 {
    // build the persistent workspace
    let workspace = match args.program.daemon_workspace() {
        Ok(workspace) => workspace,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };

    // resolve daemon service metadata
    let repository = workspace.session().repository();
    let mut endpoint = DaemonEndpoint::new(repository.layout().home.clone());

    // override the socket path when requested
    if let Some(socket) = args.socket.as_ref() {
        endpoint.socket_path = socket.clone();
    }

    // build the daemon over the already prepared workspace
    let daemon = match Daemon::with_options(workspace, endpoint, DaemonOptions::default()) {
        Ok(daemon) => daemon,
        Err(error) => {
            console::error(&format!("daemon error: {error}"));
            return 1;
        }
    };

    // serve until shutdown
    if let Err(error) = daemon.serve() {
        console::error(&format!("daemon error: {error}"));
        return 1;
    }

    0
}

/// Start the daemon.
fn run_start(args: &DaemonLifecycleArgs) -> i32 {
    // resolve the daemon endpoint without opening a repository
    let endpoint = match args.program.daemon_endpoint() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };

    // discover only the initial workspace root
    let root = match args.program.workspace_root() {
        Ok(root) => root,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };

    // build the daemon process launch
    let launch = match args.program.daemon_launch(&endpoint, root) {
        Ok(launch) => launch,
        Err(error) => {
            console::error(&format!("failed to start daemon: {error}"));
            return 1;
        }
    };
    let options = DaemonConnectOptions::default();
    let result = endpoint.start(options, launch);

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

/// Stop the daemon.
fn run_stop(args: &DaemonLifecycleArgs) -> i32 {
    // resolve the daemon endpoint without opening a repository
    let endpoint = match args.program.daemon_endpoint() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };
    let options = DaemonConnectOptions::default();

    // request shutdown and report the result
    match endpoint.stop(options) {
        Ok(()) => {
            console::info("daemon shutdown requested");
            0
        }
        Err(error) => {
            console::warn(&format!("daemon not running: {error}"));
            1
        }
    }
}

/// Report daemon status.
fn run_status(args: &DaemonLifecycleArgs) -> i32 {
    // resolve the daemon endpoint without opening a repository
    let endpoint = match args.program.daemon_endpoint() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };
    let options = DaemonConnectOptions::default();

    // probe daemon connectivity
    match endpoint.ping(options) {
        Ok(()) => {
            console::info("daemon is running");
            if let Ok(Some(metadata)) = endpoint.read_metadata() {
                console::info(&format!("websocket: {}", metadata.websocket_url));
            }
            0
        }
        Err(error) => {
            console::warn(&format!("daemon not running: {error}"));
            1
        }
    }
}
