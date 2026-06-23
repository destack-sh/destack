use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Subcommand};

use destack_daemon::{WorkspaceServer, WorkspaceServerOptions};
use destack_workspace::{
    WorkspaceConnectOptions, WorkspaceEndpoint, WorkspaceLaunch, WorkspaceServerCommand,
};

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
    // build a repository
    let repository = match args.program.open_repository() {
        Ok(repository) => repository,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };

    // resolve daemon endpoint metadata
    let mut endpoint = WorkspaceEndpoint::new(repository.layout().home.clone());

    // override the socket path when requested
    if let Some(socket) = args.socket.as_ref() {
        endpoint.socket_path = socket.clone();
    }

    // build server options
    let server_options = WorkspaceServerOptions {
        worker_limit: args.program.workers as usize,
        ..Default::default()
    };
    let server = match WorkspaceServer::with_options(repository, endpoint, server_options) {
        Ok(server) => server,
        Err(error) => {
            console::error(&format!("failed to start daemon: {error}"));
            return 1;
        }
    };

    // serve until shutdown
    if let Err(error) = server.serve() {
        console::error(&format!("daemon error: {error}"));
        return 1;
    }

    0
}

/// Start the daemon.
fn run_start(args: &DaemonLifecycleArgs) -> i32 {
    // build repository metadata
    let repository = match args.program.open_repository() {
        Ok(repository) => repository,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };
    let endpoint = WorkspaceEndpoint::new(repository.layout().home.clone());
    let launch =
        match workspace_server_launch(&args.program, &endpoint, repository.path().to_path_buf()) {
            Ok(launch) => launch,
            Err(error) => {
                console::error(&format!("failed to start daemon: {error}"));
                return 1;
            }
        };
    let options = WorkspaceConnectOptions::default();
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
    // build repository metadata
    let repository = match args.program.open_repository() {
        Ok(repository) => repository,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };
    let endpoint = WorkspaceEndpoint::new(repository.layout().home.clone());
    let options = WorkspaceConnectOptions::default();
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
    // build repository metadata
    let repository = match args.program.open_repository() {
        Ok(repository) => repository,
        Err(error) => {
            console::error(&error.to_string());
            return 1;
        }
    };
    let endpoint = WorkspaceEndpoint::new(repository.layout().home.clone());
    let options = WorkspaceConnectOptions::default();

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

/// Build a workspace server launch from CLI program settings.
fn workspace_server_launch(
    program: &ProgramArgs,
    endpoint: &WorkspaceEndpoint,
    root: PathBuf,
) -> Result<WorkspaceLaunch, std::io::Error> {
    let command = workspace_server_command(program)?;

    Ok(command.launch(endpoint, root))
}

/// Build a workspace server command from CLI program settings.
fn workspace_server_command(
    program: &ProgramArgs,
) -> Result<WorkspaceServerCommand, std::io::Error> {
    let cwd = program
        .cwd
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;
    let cache_dir = program.cache_dir.as_ref().map(|cache_dir| {
        if cache_dir.is_absolute() {
            cache_dir.clone()
        } else {
            cwd.join(cache_dir)
        }
    });
    let mut command = WorkspaceServerCommand::current_executable(workspace_server_arguments())?;

    // pass layout and cwd settings through to the workspace server
    command.home = program.home.clone();
    command.package_dir = program.package_dir.clone();
    command.cache_dir = cache_dir;
    command.manifest_path = program.manifest.clone();
    command.cwd = Some(cwd);

    Ok(command)
}

/// Return the CLI arguments that start a workspace server.
fn workspace_server_arguments() -> Vec<OsString> {
    vec![OsString::from("daemon"), OsString::from("serve")]
}
