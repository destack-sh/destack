use std::fs::{self, File, OpenOptions};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose;
use fs2::FileExt;
use tspp_core::stable_hash_bytes;
use tspp_rpc::ProtocolVersion;

use super::{DaemonEndpointError, DaemonMetadata};

/// Directory containing daemon discovery state.
const DAEMON_DIRECTORY: &str = "daemon";

/// Directory containing local daemon sockets.
const SOCKET_DIRECTORY: &str = "tspp-daemon";

/// Filesystem and network addresses for one daemon instance.
#[derive(Debug, Clone)]
pub struct DaemonEndpoint {
    /// Machine-local toolchain home.
    pub home: PathBuf,
    /// Stable endpoint instance identifier.
    pub instance_id: String,
    /// Directory containing discovery state.
    pub state_directory: PathBuf,
    /// Directory containing the local socket.
    pub socket_directory: PathBuf,
    /// Local RPC socket path.
    pub socket_path: PathBuf,
    /// Loopback WebSocket bind address.
    pub websocket_address: SocketAddr,
    /// WebSocket HTTP request path.
    pub websocket_path: String,
    /// Exclusive daemon lock path.
    pub lock_path: PathBuf,
    /// Discovery record path.
    pub metadata_path: PathBuf,
}

impl DaemonEndpoint {
    /// Create one daemon endpoint for a toolchain home.
    pub fn new(home: PathBuf) -> Self {
        let instance_id = format!(
            "language-v{}-rpc{}",
            env!("CARGO_PKG_VERSION"),
            ProtocolVersion::CURRENT.0,
        );
        let state_directory = home.join(DAEMON_DIRECTORY).join(&instance_id);
        let socket_directory = std::env::temp_dir().join(SOCKET_DIRECTORY);
        let socket_path = socket_directory.join(Self::socket_name(&home, &instance_id));

        Self {
            home,
            instance_id,
            lock_path: state_directory.join("daemon.lock"),
            metadata_path: state_directory.join("daemon.json"),
            state_directory,
            socket_directory,
            socket_path,
            websocket_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            websocket_path: "/rpc".to_string(),
        }
    }

    /// Acquire exclusive ownership of this endpoint.
    pub fn lock(&self) -> Result<DaemonLock, DaemonEndpointError> {
        fs::create_dir_all(&self.state_directory)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        if let Err(error) = file.try_lock_exclusive() {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(DaemonEndpointError::AlreadyRunning);
            }

            return Err(error.into());
        }

        Ok(DaemonLock { file })
    }

    /// Remove a stale local socket before binding.
    pub fn clear_socket(&self) -> Result<(), DaemonEndpointError> {
        fs::create_dir_all(&self.socket_directory)?;
        if let Err(error) = fs::remove_file(&self.socket_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }

        Ok(())
    }

    /// Read the running daemon discovery record when present.
    pub fn read_metadata(&self) -> Result<Option<DaemonMetadata>, DaemonEndpointError> {
        let contents = match fs::read_to_string(&self.metadata_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let metadata = serde_json::from_str(&contents)?;

        Ok(Some(metadata))
    }

    /// Write the running daemon discovery record.
    pub fn write_metadata(&self, metadata: &DaemonMetadata) -> Result<(), DaemonEndpointError> {
        fs::create_dir_all(&self.state_directory)?;
        let contents = serde_json::to_string_pretty(metadata)?;
        fs::write(&self.metadata_path, contents)?;

        Ok(())
    }

    /// Remove the daemon discovery record when present.
    pub fn remove_metadata(&self) -> Result<(), DaemonEndpointError> {
        if let Err(error) = fs::remove_file(&self.metadata_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }

        Ok(())
    }

    /// Create a fresh WebSocket bearer token.
    pub fn websocket_token(&self) -> Result<String, DaemonEndpointError> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(DaemonEndpointError::Random)?;

        Ok(general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Return one authenticated browser WebSocket URL.
    pub fn websocket_url(&self, address: SocketAddr, token: &str) -> String {
        format!("ws://{address}{}?token={token}", self.websocket_path)
    }

    /// Return a compact deterministic socket filename.
    fn socket_name(home: &Path, instance_id: &str) -> String {
        let mut identity = home.as_os_str().as_encoded_bytes().to_vec();
        identity.extend_from_slice(instance_id.as_bytes());
        let hash = stable_hash_bytes(&identity);

        format!("ds-{hash:016x}.sock")
    }
}

/// Exclusive ownership of one daemon endpoint.
#[derive(Debug)]
pub struct DaemonLock {
    /// Locked file handle.
    file: File,
}

impl DaemonLock {
    /// Return the locked file handle.
    pub const fn file(&self) -> &File {
        &self.file
    }
}
