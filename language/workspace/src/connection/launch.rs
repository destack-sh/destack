use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use super::WorkspaceEndpoint;

/// Command used to spawn a workspace server.
#[derive(Debug, Clone)]
pub struct WorkspaceServerCommand {
    /// Executable used to spawn the workspace server.
    pub executable: PathBuf,
    /// Arguments passed before workspace server options.
    pub arguments: Vec<OsString>,
    /// Destack home directory override for the workspace server.
    pub home: Option<PathBuf>,
    /// Package directory override for the workspace server.
    pub package_dir: Option<PathBuf>,
    /// Workspace cache directory override for the workspace server.
    pub cache_dir: Option<PathBuf>,
    /// Manifest path override for the workspace server.
    pub manifest_path: Option<PathBuf>,
    /// Cwd override for the workspace server.
    pub cwd: Option<PathBuf>,
}

impl WorkspaceServerCommand {
    /// Build a workspace server command from the current executable.
    pub fn current_executable(arguments: Vec<OsString>) -> Result<Self, std::io::Error> {
        let executable = std::env::current_exe()?;

        Ok(Self {
            executable,
            arguments,
            home: None,
            package_dir: None,
            cache_dir: None,
            manifest_path: None,
            cwd: None,
        })
    }

    /// Build endpoint-specific launch values.
    pub fn launch(&self, endpoint: &WorkspaceEndpoint, root: PathBuf) -> WorkspaceLaunch {
        let mut launch = WorkspaceLaunch::new(
            endpoint,
            self.executable.clone(),
            self.arguments.clone(),
            root,
        );
        launch.home = self.home.clone();
        launch.package_dir = self.package_dir.clone();
        launch.cache_dir = self.cache_dir.clone();
        launch.manifest_path = self.manifest_path.clone();
        launch.cwd = self.cwd.clone();

        launch
    }
}

/// Launch values for spawning a workspace server process.
#[derive(Debug, Clone)]
pub struct WorkspaceLaunch {
    /// Executable used to spawn the workspace server.
    pub executable: PathBuf,
    /// Arguments passed before workspace server options.
    pub arguments: Vec<OsString>,
    /// Root for the workspace server.
    pub root: PathBuf,
    /// Socket path for the workspace server.
    pub socket_path: PathBuf,
    /// Destack home directory override for the workspace server.
    pub home: Option<PathBuf>,
    /// Package directory override for the workspace server.
    pub package_dir: Option<PathBuf>,
    /// Workspace cache directory override for the workspace server.
    pub cache_dir: Option<PathBuf>,
    /// Manifest path override for the workspace server.
    pub manifest_path: Option<PathBuf>,
    /// Cwd override for the workspace server.
    pub cwd: Option<PathBuf>,
}

impl WorkspaceLaunch {
    /// Build a workspace server launch from one executable.
    pub fn new(
        endpoint: &WorkspaceEndpoint,
        executable: PathBuf,
        arguments: Vec<OsString>,
        root: PathBuf,
    ) -> Self {
        Self {
            executable,
            arguments,
            root,
            socket_path: endpoint.socket_path.clone(),
            home: None,
            package_dir: None,
            cache_dir: None,
            manifest_path: None,
            cwd: None,
        }
    }

    /// Build a workspace server launch from the current executable.
    pub fn for_current_executable(
        endpoint: &WorkspaceEndpoint,
        arguments: Vec<OsString>,
        root: PathBuf,
    ) -> Result<Self, std::io::Error> {
        let command = WorkspaceServerCommand::current_executable(arguments)?;

        Ok(command.launch(endpoint, root))
    }

    /// Build a command to launch a workspace server.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(&self.arguments);
        command.arg("--workspace").arg(&self.root);
        command.arg("--socket").arg(&self.socket_path);

        // add layout overrides
        if let Some(home) = self.home.as_ref() {
            command.arg("--home").arg(home);
        }
        if let Some(package_dir) = self.package_dir.as_ref() {
            command.arg("--package-dir").arg(package_dir);
        }
        if let Some(cache_dir) = self.cache_dir.as_ref() {
            command.arg("--cache-dir").arg(cache_dir);
        }
        if let Some(manifest_path) = self.manifest_path.as_ref() {
            command.arg("--manifest").arg(manifest_path);
        }
        if let Some(cwd) = self.cwd.as_ref() {
            command.arg("--cwd").arg(cwd);
        }

        command
    }
}
