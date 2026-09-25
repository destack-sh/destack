use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use super::DaemonEndpoint;

/// Reusable command description for launching a daemon.
#[derive(Debug, Clone)]
pub struct DaemonLaunchCommand {
    /// Executable to launch.
    pub executable: PathBuf,
    /// Arguments preceding daemon options.
    pub arguments: Vec<OsString>,
    /// Toolchain home override.
    pub home: Option<PathBuf>,
    /// Package directory override.
    pub package_directory: Option<PathBuf>,
    /// Cache directory override.
    pub cache_directory: Option<PathBuf>,
    /// Manifest path override.
    pub manifest: Option<PathBuf>,
    /// Process working directory.
    pub current_directory: Option<PathBuf>,
}

impl DaemonLaunchCommand {
    /// Create a daemon command from the current executable.
    pub fn current(arguments: Vec<OsString>) -> Result<Self, std::io::Error> {
        Ok(Self {
            executable: std::env::current_exe()?,
            arguments,
            home: None,
            package_directory: None,
            cache_directory: None,
            manifest: None,
            current_directory: None,
        })
    }

    /// Bind this command to one endpoint and workspace root.
    pub fn launch(&self, endpoint: &DaemonEndpoint, root: PathBuf) -> DaemonLaunch {
        DaemonLaunch {
            executable: self.executable.clone(),
            arguments: self.arguments.clone(),
            root,
            socket_path: endpoint.socket_path.clone(),
            home: self.home.clone(),
            package_directory: self.package_directory.clone(),
            cache_directory: self.cache_directory.clone(),
            manifest: self.manifest.clone(),
            current_directory: self.current_directory.clone(),
        }
    }
}

/// Exact process invocation for one daemon endpoint.
#[derive(Debug, Clone)]
pub struct DaemonLaunch {
    /// Executable to launch.
    pub executable: PathBuf,
    /// Arguments preceding daemon options.
    pub arguments: Vec<OsString>,
    /// Initial workspace root.
    pub root: PathBuf,
    /// Local RPC socket path.
    pub socket_path: PathBuf,
    /// Toolchain home override.
    pub home: Option<PathBuf>,
    /// Package directory override.
    pub package_directory: Option<PathBuf>,
    /// Cache directory override.
    pub cache_directory: Option<PathBuf>,
    /// Manifest path override.
    pub manifest: Option<PathBuf>,
    /// Process working directory.
    pub current_directory: Option<PathBuf>,
}

impl DaemonLaunch {
    /// Build the exact process command.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(&self.arguments);
        command.arg("--workspace").arg(&self.root);
        command.arg("--socket").arg(&self.socket_path);

        if let Some(home) = self.home.as_ref() {
            command.arg("--home").arg(home);
        }
        if let Some(directory) = self.package_directory.as_ref() {
            command.arg("--package-dir").arg(directory);
        }
        if let Some(directory) = self.cache_directory.as_ref() {
            command.arg("--cache-dir").arg(directory);
        }
        if let Some(manifest) = self.manifest.as_ref() {
            command.arg("--manifest").arg(manifest);
        }
        if let Some(directory) = self.current_directory.as_ref() {
            command.arg("--cwd").arg(directory);
        }

        command
    }
}
