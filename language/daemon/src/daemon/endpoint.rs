use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::protocol::PROTOCOL_VERSION;

use super::constants::{ENDPOINT_DIRECTORY, LOCK_FILE, METADATA_FILE, SOCKET_DIRECTORY};
use super::metadata::DaemonMetadata;

/// Daemon endpoint paths for one user and toolchain.
#[derive(Debug, Clone)]
pub struct DaemonEndpoint {
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Stable toolchain instance id.
    pub instance_id: String,
    /// Directory for daemon endpoint files.
    pub dir: PathBuf,
    /// Directory for daemon socket files.
    pub socket_dir: PathBuf,
    /// Socket path for the daemon endpoint.
    pub socket_path: PathBuf,
    /// Lock file path for the daemon endpoint.
    pub lock_path: PathBuf,
    /// Metadata path for the daemon endpoint.
    pub metadata_path: PathBuf,
}

impl DaemonEndpoint {
    /// Create daemon endpoint paths for one Destack home.
    pub fn new(home: PathBuf) -> Self {
        // build stable instance identity from the running toolchain
        let instance_id = toolchain_instance_id();

        // build endpoint paths
        let dir = home.join(ENDPOINT_DIRECTORY).join(&instance_id);
        let socket_dir = std::env::temp_dir().join(SOCKET_DIRECTORY);
        let socket_path = socket_dir.join(endpoint_socket_name(&home, &instance_id));
        let lock_path = dir.join(LOCK_FILE);
        let metadata_path = dir.join(METADATA_FILE);

        // return the endpoint layout
        Self {
            home,
            instance_id,
            dir,
            socket_dir,
            socket_path,
            lock_path,
            metadata_path,
        }
    }

    /// Ensure the daemon directory exists.
    pub fn ensure_dir(&self) -> Result<(), DaemonEndpointError> {
        fs::create_dir_all(&self.dir).map_err(DaemonEndpointError::Io)
    }

    /// Ensure the socket directory exists.
    pub fn ensure_socket_dir(&self) -> Result<(), DaemonEndpointError> {
        fs::create_dir_all(&self.socket_dir).map_err(DaemonEndpointError::Io)
    }

    /// Acquire the daemon lock for this endpoint.
    pub fn lock(&self) -> Result<DaemonLock, DaemonEndpointError> {
        // ensure the daemon directory exists
        self.ensure_dir()?;

        // open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(DaemonEndpointError::Io)?;

        // acquire the exclusive lock
        if let Err(error) = FileExt::try_lock_exclusive(&file) {
            let error: std::io::Error = error;
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(DaemonEndpointError::AlreadyRunning);
            }
            return Err(DaemonEndpointError::Io(error));
        }

        // return the lock guard
        Ok(DaemonLock {
            _file: file,
            path: self.lock_path.clone(),
        })
    }

    /// Remove any stale socket path before binding.
    pub fn clear_socket_path(&self) -> Result<(), DaemonEndpointError> {
        // ensure the socket directory exists
        self.ensure_socket_dir()?;

        // remove any stale socket file
        if let Err(error) = fs::remove_file(&self.socket_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(DaemonEndpointError::Io(error));
        }

        // indicate success
        Ok(())
    }

    /// Read daemon metadata from disk.
    pub fn read_metadata(&self) -> Result<Option<DaemonMetadata>, DaemonEndpointError> {
        // load metadata contents when present
        let contents = match fs::read_to_string(&self.metadata_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(DaemonEndpointError::Io(error)),
        };

        // decode the metadata json
        let metadata = serde_json::from_str(&contents).map_err(DaemonEndpointError::Serde)?;

        // return the parsed metadata
        Ok(Some(metadata))
    }

    /// Write daemon metadata to disk.
    pub fn write_metadata(&self, metadata: &DaemonMetadata) -> Result<(), DaemonEndpointError> {
        // ensure the daemon directory exists
        self.ensure_dir()?;

        // encode the metadata json
        let contents =
            serde_json::to_string_pretty(metadata).map_err(DaemonEndpointError::Serde)?;

        // write the metadata file
        fs::write(&self.metadata_path, contents).map_err(DaemonEndpointError::Io)
    }

    /// Remove daemon metadata from disk.
    pub fn remove_metadata(&self) -> Result<(), DaemonEndpointError> {
        // remove the metadata file when present
        if let Err(error) = fs::remove_file(&self.metadata_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(DaemonEndpointError::Io(error));
        }

        // indicate success
        Ok(())
    }
}

/// Lock guard for a daemon endpoint.
#[derive(Debug)]
pub struct DaemonLock {
    /// Locked file handle for the daemon endpoint.
    _file: File,
    /// Path to the lock file.
    pub path: PathBuf,
}

/// Errors for daemon endpoint operations.
#[derive(Debug)]
pub enum DaemonEndpointError {
    /// The daemon is already running.
    AlreadyRunning,
    /// Io error.
    Io(std::io::Error),
    /// Serialization error.
    Serde(serde_json::Error),
}

impl std::fmt::Display for DaemonEndpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonEndpointError::AlreadyRunning => {
                write!(f, "daemon endpoint already running")
            }
            DaemonEndpointError::Io(error) => write!(f, "daemon endpoint io error: {error}"),
            DaemonEndpointError::Serde(error) => {
                write!(f, "daemon endpoint metadata error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonEndpointError {}

impl From<std::io::Error> for DaemonEndpointError {
    fn from(error: std::io::Error) -> Self {
        DaemonEndpointError::Io(error)
    }
}

impl From<serde_json::Error> for DaemonEndpointError {
    fn from(error: serde_json::Error) -> Self {
        DaemonEndpointError::Serde(error)
    }
}

/// Return the stable instance id for this daemon toolchain.
fn toolchain_instance_id() -> String {
    // combine protocol and package version
    let version = env!("CARGO_PKG_VERSION");
    let protocol = PROTOCOL_VERSION;

    format!(
        "language-v{version}-p{}.{}.{}",
        protocol.major, protocol.minor, protocol.patch
    )
}

/// Return the socket name for one home and toolchain instance.
fn endpoint_socket_name(home: &Path, instance_id: &str) -> String {
    // include home and instance in a compact unix socket safe name
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    home.hash(&mut hasher);
    instance_id.hash(&mut hasher);
    let home_hash = hasher.finish();

    format!("ds-{home_hash:016x}.sock")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_artifact::DiskCacheStore;
    use destack_repository::{
        DestackLayout, DestackLayoutOverride, Environment, Repository, Settings,
    };
    use destack_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};

    use super::{DaemonEndpoint, DaemonEndpointError};

    /// Build a repository with an isolated daemon home.
    fn test_repository(prefix: &str) -> Arc<Repository> {
        // create a temporary workspace and isolated home
        let root = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let overrides = DestackLayoutOverride {
            home: Some(root.root().join("home")),
            ..DestackLayoutOverride::default()
        };
        let layout = DestackLayout::resolve(
            root.root(),
            root.root(),
            &environment,
            &Settings::default(),
            &overrides,
            None,
        );

        Arc::new(Repository::new(
            root.root().to_path_buf(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            environment,
            Settings::default(),
            layout,
        ))
    }

    /// Generate stable daemon endpoint paths for one Destack home.
    #[test]
    fn test_daemon_endpoint_paths_are_stable() {
        // build two endpoints for the same home
        let repository = test_repository("daemon_endpoint_paths");
        let home = repository.layout().home.clone();
        let endpoint = DaemonEndpoint::new(home.clone());

        let endpoint_again = DaemonEndpoint::new(home);

        // assertion block
        assert_eq!(endpoint.dir, endpoint_again.dir);
        assert_eq!(endpoint.socket_path, endpoint_again.socket_path);
    }

    /// Reject acquiring a second daemon endpoint lock.
    #[test]
    fn test_daemon_endpoint_lock_rejects_second_acquire() {
        // create a daemon endpoint for a temporary home
        let repository = test_repository("daemon_endpoint_lock");
        let endpoint = DaemonEndpoint::new(repository.layout().home.clone());

        // acquire the first lock
        let _guard = endpoint.lock().expect("lock should succeed");

        // assertion block
        let second = endpoint.lock();
        assert!(matches!(second, Err(DaemonEndpointError::AlreadyRunning)));
    }
}
