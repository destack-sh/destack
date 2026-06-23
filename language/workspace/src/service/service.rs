use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose;
use fs2::FileExt;

use crate::protocol::PROTOCOL_VERSION;

use super::constants::{LOCK_FILE, METADATA_FILE, SERVICE_DIRECTORY, SOCKET_DIRECTORY};
use super::metadata::Metadata;

/// Workspace service paths for one user and command.
#[derive(Debug, Clone)]
pub struct Service {
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Stable command instance id.
    pub instance_id: String,
    /// Directory for workspace service files.
    pub dir: PathBuf,
    /// Directory for workspace socket files.
    pub socket_dir: PathBuf,
    /// Socket path for the workspace service.
    pub socket_path: PathBuf,
    /// Local WebSocket bind address for browser workspace connections.
    pub websocket_addr: SocketAddr,
    /// Local WebSocket path for browser workspace connections.
    pub websocket_path: String,
    /// Lock file path for the workspace service.
    pub lock_path: PathBuf,
    /// Metadata path for the workspace service.
    pub metadata_path: PathBuf,
}

impl Service {
    /// Create workspace service paths for one Destack home.
    pub fn new(home: PathBuf) -> Self {
        // build stable instance identity from the running command
        let instance_id = Self::instance_id();

        // build service paths
        let dir = home.join(SERVICE_DIRECTORY).join(&instance_id);
        let socket_dir = std::env::temp_dir().join(SOCKET_DIRECTORY);
        let socket_path = socket_dir.join(Self::socket_name(&home, &instance_id));
        let websocket_addr = Self::websocket_addr();
        let websocket_path = "/workspace".to_string();
        let lock_path = dir.join(LOCK_FILE);
        let metadata_path = dir.join(METADATA_FILE);

        // return the service layout
        Self {
            home,
            instance_id,
            dir,
            socket_dir,
            socket_path,
            websocket_addr,
            websocket_path,
            lock_path,
            metadata_path,
        }
    }

    /// Ensure the workspace service directory exists.
    pub fn ensure_dir(&self) -> Result<(), ServiceError> {
        fs::create_dir_all(&self.dir).map_err(ServiceError::Io)
    }

    /// Ensure the socket directory exists.
    pub fn ensure_socket_dir(&self) -> Result<(), ServiceError> {
        fs::create_dir_all(&self.socket_dir).map_err(ServiceError::Io)
    }

    /// Acquire the workspace service lock.
    pub fn lock(&self) -> Result<Lock, ServiceError> {
        // ensure the workspace service directory exists
        self.ensure_dir()?;

        // open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(ServiceError::Io)?;

        // acquire the exclusive lock
        if let Err(error) = FileExt::try_lock_exclusive(&file) {
            let error: std::io::Error = error;
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(ServiceError::AlreadyRunning);
            }
            return Err(ServiceError::Io(error));
        }

        // return the lock guard
        Ok(Lock {
            _file: file,
            path: self.lock_path.clone(),
        })
    }

    /// Remove any stale socket path before binding.
    pub fn clear_socket_path(&self) -> Result<(), ServiceError> {
        // ensure the socket directory exists
        self.ensure_socket_dir()?;

        // remove any stale socket file
        if let Err(error) = fs::remove_file(&self.socket_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(ServiceError::Io(error));
        }

        // indicate success
        Ok(())
    }

    /// Read workspace service metadata from disk.
    pub fn read_metadata(&self) -> Result<Option<Metadata>, ServiceError> {
        // load metadata contents when present
        let contents = match fs::read_to_string(&self.metadata_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(ServiceError::Io(error)),
        };

        // decode the metadata json
        let metadata = serde_json::from_str(&contents).map_err(ServiceError::Serde)?;

        // return the parsed metadata
        Ok(Some(metadata))
    }

    /// Write workspace service metadata to disk.
    pub fn write_metadata(&self, metadata: &Metadata) -> Result<(), ServiceError> {
        // ensure the workspace service directory exists
        self.ensure_dir()?;

        // encode the metadata json
        let contents = serde_json::to_string_pretty(metadata).map_err(ServiceError::Serde)?;

        // write the metadata file
        fs::write(&self.metadata_path, contents).map_err(ServiceError::Io)
    }

    /// Remove workspace service metadata from disk.
    pub fn remove_metadata(&self) -> Result<(), ServiceError> {
        // remove the metadata file when present
        if let Err(error) = fs::remove_file(&self.metadata_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(ServiceError::Io(error));
        }

        // indicate success
        Ok(())
    }

    /// Create a fresh WebSocket bearer token.
    pub fn create_websocket_token(&self) -> Result<String, ServiceError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(ServiceError::Random)?;

        Ok(general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Return the authenticated WebSocket URL for one token.
    pub fn websocket_url(&self, addr: SocketAddr, token: &str) -> String {
        format!("ws://{}{}?token={}", addr, self.websocket_path, token)
    }

    /// Return the stable instance id for this workspace protocol.
    fn instance_id() -> String {
        // combine protocol and package version
        let version = env!("CARGO_PKG_VERSION");
        let protocol = PROTOCOL_VERSION;

        format!(
            "language-v{version}-p{}.{}.{}",
            protocol.major, protocol.minor, protocol.patch
        )
    }

    /// Return the socket name for one home and instance.
    fn socket_name(home: &Path, instance_id: &str) -> String {
        // include home and instance in a compact unix socket safe name
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        home.hash(&mut hasher);
        instance_id.hash(&mut hasher);
        let home_hash = hasher.finish();

        format!("ds-{home_hash:016x}.sock")
    }

    /// Return the WebSocket bind address for browser workspace connections.
    fn websocket_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }
}

/// Lock guard for a workspace service.
#[derive(Debug)]
pub struct Lock {
    /// Locked file handle for the workspace service.
    _file: File,
    /// Path to the lock file.
    pub path: PathBuf,
}

/// Errors for workspace service operations.
#[derive(Debug)]
pub enum ServiceError {
    /// The workspace service is already running.
    AlreadyRunning,
    /// Io error.
    Io(std::io::Error),
    /// Serialization error.
    Serde(serde_json::Error),
    /// System clock error.
    Time(std::time::SystemTimeError),
    /// Randomness error.
    Random(getrandom::Error),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::AlreadyRunning => {
                write!(f, "workspace service already running")
            }
            ServiceError::Io(error) => write!(f, "workspace service io error: {error}"),
            ServiceError::Serde(error) => {
                write!(f, "workspace service metadata error: {error}")
            }
            ServiceError::Time(error) => {
                write!(f, "workspace service time error: {error}")
            }
            ServiceError::Random(error) => {
                write!(f, "workspace service random error: {error}")
            }
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        ServiceError::Io(error)
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(error: serde_json::Error) -> Self {
        ServiceError::Serde(error)
    }
}
