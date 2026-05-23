use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use destack_core::stable_hash_bytes;
use destack_workspace::Repository;

use crate::protocol::{MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolRange};

/// Directory name for daemon instance data.
pub const DAEMON_DIR_NAME: &str = "daemon";
/// Lock file name for daemon instances.
pub const DAEMON_LOCK_FILE_NAME: &str = "daemon.lock";
/// Metadata file name for daemon instances.
pub const DAEMON_METADATA_FILE_NAME: &str = "daemon.json";
/// Socket file name for daemon instances.
pub const DAEMON_SOCKET_FILE_NAME: &str = "daemon.sock";

/// Daemon instance paths for a root.
#[derive(Debug, Clone)]
pub struct DaemonInstance {
    /// Root for this instance.
    pub root: PathBuf,
    /// Cache root used for daemon data.
    pub cache_root: PathBuf,
    /// Stable root hash for this instance.
    pub root_id: String,
    /// Directory for daemon instance files.
    pub dir: PathBuf,
    /// Directory for daemon socket files.
    pub socket_dir: PathBuf,
    /// Socket path for the daemon instance.
    pub socket_path: PathBuf,
    /// Lock file path for the daemon instance.
    pub lock_path: PathBuf,
    /// Metadata path for the daemon instance.
    pub metadata_path: PathBuf,
}

impl DaemonInstance {
    /// Create daemon instance paths for a root and cache root.
    pub fn new(root: PathBuf, cache_root: PathBuf) -> Self {
        // canonicalize the root
        let stable_root = fs::canonicalize(&root).unwrap_or(root.clone());

        // hash the root for the instance id
        let root_id = hash_root(&stable_root);

        // build instance paths
        let dir = cache_root.join(DAEMON_DIR_NAME).join(&root_id);
        let socket_dir = std::env::temp_dir().join("destack").join("daemon");
        let socket_path = socket_dir.join(format!("{root_id}.sock"));
        let lock_path = dir.join(DAEMON_LOCK_FILE_NAME);
        let metadata_path = dir.join(DAEMON_METADATA_FILE_NAME);

        // return the instance layout
        Self {
            root,
            cache_root,
            root_id,
            dir,
            socket_dir,
            socket_path,
            lock_path,
            metadata_path,
        }
    }

    /// Build a daemon instance from a repository.
    pub fn from_repository(repository: &Repository) -> Self {
        // resolve roots from the repository
        let root = repository.workspace_root().to_path_buf();
        let cache_root = repository.cache_directory();

        // build the daemon instance
        Self::new(root, cache_root)
    }

    /// Ensure the daemon directory exists.
    pub fn ensure_dir(&self) -> Result<(), DaemonInstanceError> {
        fs::create_dir_all(&self.dir).map_err(DaemonInstanceError::Io)
    }

    /// Ensure the socket directory exists.
    pub fn ensure_socket_dir(&self) -> Result<(), DaemonInstanceError> {
        fs::create_dir_all(&self.socket_dir).map_err(DaemonInstanceError::Io)
    }

    /// Acquire the daemon lock for this instance.
    pub fn lock(&self) -> Result<DaemonInstanceLock, DaemonInstanceError> {
        // ensure the daemon directory exists
        self.ensure_dir()?;

        // open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(DaemonInstanceError::Io)?;

        // acquire the exclusive lock
        if let Err(error) = FileExt::try_lock_exclusive(&file) {
            let error: std::io::Error = error;
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(DaemonInstanceError::AlreadyRunning);
            }
            return Err(DaemonInstanceError::Io(error));
        }

        // return the lock guard
        Ok(DaemonInstanceLock {
            _file: file,
            path: self.lock_path.clone(),
        })
    }

    /// Remove any stale socket path before binding.
    pub fn clear_socket_path(&self) -> Result<(), DaemonInstanceError> {
        // ensure the socket directory exists
        self.ensure_socket_dir()?;

        // remove any stale socket file
        if let Err(error) = fs::remove_file(&self.socket_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(DaemonInstanceError::Io(error));
        }

        // indicate success
        Ok(())
    }

    /// Read daemon metadata from disk.
    pub fn read_metadata(&self) -> Result<Option<DaemonMetadata>, DaemonInstanceError> {
        // load metadata contents when present
        let contents = match fs::read_to_string(&self.metadata_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(DaemonInstanceError::Io(error)),
        };

        // decode the metadata json
        let metadata = serde_json::from_str(&contents).map_err(DaemonInstanceError::Serde)?;

        // return the parsed metadata
        Ok(Some(metadata))
    }

    /// Write daemon metadata to disk.
    pub fn write_metadata(&self, metadata: &DaemonMetadata) -> Result<(), DaemonInstanceError> {
        // ensure the daemon directory exists
        self.ensure_dir()?;

        // encode the metadata json
        let contents =
            serde_json::to_string_pretty(metadata).map_err(DaemonInstanceError::Serde)?;

        // write the metadata file
        fs::write(&self.metadata_path, contents).map_err(DaemonInstanceError::Io)
    }

    /// Remove daemon metadata from disk.
    pub fn remove_metadata(&self) -> Result<(), DaemonInstanceError> {
        // remove the metadata file when present
        if let Err(error) = fs::remove_file(&self.metadata_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(DaemonInstanceError::Io(error));
        }

        // indicate success
        Ok(())
    }
}

/// Lock guard for a daemon instance.
#[derive(Debug)]
pub struct DaemonInstanceLock {
    /// Locked file handle for the daemon instance.
    _file: File,
    /// Path to the lock file.
    pub path: PathBuf,
}

/// Metadata stored for a running daemon instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMetadata {
    /// Metadata schema version.
    pub schema_version: u32,
    /// Root for the daemon.
    pub root: PathBuf,
    /// Cache root for daemon data.
    pub cache_root: PathBuf,
    /// Socket path for daemon connections.
    pub socket_path: PathBuf,
    /// Daemon process id.
    pub pid: u32,
    /// Daemon protocol range.
    pub protocol: ProtocolRange,
    /// Daemon version string.
    pub version: String,
    /// Unix timestamp for daemon start.
    pub started_at: u64,
}

/// Launch configuration for spawning a daemon process.
#[derive(Debug, Clone)]
pub struct DaemonLaunchConfig {
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

impl DaemonLaunchConfig {
    /// Build a daemon launch config from an instance.
    pub fn for_instance(instance: &DaemonInstance) -> Self {
        // build default launch config values
        Self {
            root: instance.root.clone(),
            socket_path: instance.socket_path.clone(),
            home: None,
            package_dir: None,
            cache_dir: None,
            manifest_path: None,
            cwd: None,
        }
    }

    /// Build a command to launch a daemon from the current executable.
    pub fn command_from_current_exe(&self) -> Result<Command, std::io::Error> {
        // resolve the current executable path
        let executable = std::env::current_exe()?;

        // build the daemon command
        Ok(self.command(&executable))
    }

    /// Build a command to launch a daemon from an executable.
    pub fn command(&self, executable: &Path) -> Command {
        // build the base daemon command
        let mut command = Command::new(executable);
        command.arg("daemon").arg("serve");
        command.arg("--workspace").arg(&self.root);
        command.arg("--socket").arg(&self.socket_path);

        // append optional overrides
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

        // return the configured command
        command
    }
}

impl DaemonMetadata {
    /// Create metadata for a daemon instance.
    pub fn new(instance: &DaemonInstance) -> Self {
        // build the metadata payload
        Self {
            schema_version: 1,
            root: instance.root.clone(),
            cache_root: instance.cache_root.clone(),
            socket_path: instance.socket_path.clone(),
            pid: std::process::id(),
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: current_unix_seconds(),
        }
    }
}

/// Errors for daemon instance operations.
#[derive(Debug)]
pub enum DaemonInstanceError {
    /// The daemon is already running.
    AlreadyRunning,
    /// Io error.
    Io(std::io::Error),
    /// Serialization error.
    Serde(serde_json::Error),
}

impl std::fmt::Display for DaemonInstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonInstanceError::AlreadyRunning => {
                write!(f, "daemon instance already running")
            }
            DaemonInstanceError::Io(error) => write!(f, "daemon instance io error: {error}"),
            DaemonInstanceError::Serde(error) => {
                write!(f, "daemon instance metadata error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonInstanceError {}

impl From<std::io::Error> for DaemonInstanceError {
    fn from(error: std::io::Error) -> Self {
        DaemonInstanceError::Io(error)
    }
}

impl From<serde_json::Error> for DaemonInstanceError {
    fn from(error: serde_json::Error) -> Self {
        DaemonInstanceError::Serde(error)
    }
}

/// Hash a root into a stable instance id.
fn hash_root(root: &Path) -> String {
    // hash the root string
    let root_bytes = root.to_string_lossy();
    let hash = stable_hash_bytes(root_bytes.as_bytes());

    // format the hash as hex
    format!("{hash:016x}")
}

/// Return the current unix timestamp in seconds.
fn current_unix_seconds() -> u64 {
    // return the current unix timestamp in seconds
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_artifact::DiskCacheStore;
    use destack_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};
    use destack_workspace::{
        DestackLayout, DestackLayoutOverride, Environment, Repository, Settings,
    };

    use super::{DaemonInstance, DaemonInstanceError};

    /// Generate stable daemon instance paths for a root.
    #[test]
    fn test_daemon_instance_paths_are_stable() {
        // build two instances for the same root
        let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_instance_paths");
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            root.root(),
            root.root(),
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let repository = Arc::new(Repository::new(
            root.root().to_path_buf(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            environment,
            Settings::default(),
            layout,
        ));
        let cache_root = repository.cache_directory();
        let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root.clone());

        let instance_again = DaemonInstance::new(root.root().to_path_buf(), cache_root);

        // assertion block
        assert_eq!(instance.dir, instance_again.dir);
        assert_eq!(instance.socket_path, instance_again.socket_path);
    }

    /// Reject acquiring a second daemon instance lock.
    #[test]
    fn test_daemon_instance_lock_rejects_second_acquire() {
        // create a daemon instance for a temporary root
        let root = TemporaryPhysicalFileSystem::new_with_prefix("daemon_instance_lock");
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            root.root(),
            root.root(),
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let repository = Arc::new(Repository::new(
            root.root().to_path_buf(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            environment,
            Settings::default(),
            layout,
        ));
        let cache_root = repository.cache_directory();
        let instance = DaemonInstance::new(root.root().to_path_buf(), cache_root);

        // acquire the first lock
        let _guard = instance.lock().expect("lock should succeed");

        // assertion block
        let second = instance.lock();
        assert!(matches!(second, Err(DaemonInstanceError::AlreadyRunning)));
    }
}
