use std::path::{Path, PathBuf};
use std::process::Command;

use super::DaemonEndpoint;

/// Launch values for spawning a daemon process.
#[derive(Debug, Clone)]
pub struct DaemonLaunch {
    /// Root for the daemon.
    pub root: PathBuf,
    /// Socket path for the daemon.
    pub socket_path: PathBuf,
    /// Destack home directory override for the daemon.
    pub home: Option<PathBuf>,
    /// Package directory override for the daemon.
    pub package_dir: Option<PathBuf>,
    /// Workspace cache directory override for the daemon.
    pub cache_dir: Option<PathBuf>,
    /// Manifest path override for the daemon.
    pub manifest_path: Option<PathBuf>,
    /// Cwd override for the daemon.
    pub cwd: Option<PathBuf>,
}

impl DaemonLaunch {
    /// Build a daemon launch from an endpoint and initial root.
    pub fn for_endpoint(endpoint: &DaemonEndpoint, root: PathBuf) -> Self {
        Self {
            root,
            socket_path: endpoint.socket_path.clone(),
            home: None,
            package_dir: None,
            cache_dir: None,
            manifest_path: None,
            cwd: None,
        }
    }

    /// Build a command to launch a daemon from the current executable.
    pub fn command_from_current_exe(&self) -> Result<Command, std::io::Error> {
        let executable = std::env::current_exe()?;

        Ok(self.command(&executable))
    }

    /// Build a command to launch a daemon from an executable.
    pub fn command(&self, executable: &Path) -> Command {
        let mut command = Command::new(executable);
        command.arg("daemon").arg("serve");
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
