use std::{error, fmt};

use destack_repository::BlobStoreError;
use destack_rpc::{
    ConnectionError, IpcError, RegistryError, ServerError, ServiceSchemaError, TransportError,
    WebSocketError,
};
use destack_source::FileWatchError;
use destack_workspace as workspace;

use crate::DaemonEndpointError;

/// Failure while serving daemon connections.
#[derive(Debug)]
pub enum DaemonError {
    /// Endpoint discovery or local transport failed.
    Endpoint(DaemonEndpointError),
    /// WebSocket listener failed.
    WebSocket(WebSocketError),
    /// RPC service declaration failed.
    Schema(ServiceSchemaError),
    /// RPC service registration failed.
    Registry(RegistryError),
    /// RPC connection failed.
    Connection(ServerError),
    /// Physical workspace observation failed.
    Watch(FileWatchError),
    /// Immutable Blob storage failed.
    Blob(BlobStoreError),
    /// Workspace initialization failed.
    Workspace(workspace::Error),
    /// A daemon thread panicked.
    Thread,
    /// Multiple failures occurred while terminating daemon owners.
    Shutdown {
        /// Serving and cleanup failures in observation order.
        failures: Vec<DaemonError>,
    },
}

impl DaemonError {
    /// Combine independently observed daemon failures.
    pub(crate) fn combine(mut failures: Vec<Self>) -> Option<Self> {
        match failures.len() {
            0 => None,
            1 => failures.pop(),
            _ => Some(Self::Shutdown { failures }),
        }
    }
}

impl fmt::Display for DaemonError {
    /// Format this daemon failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint(error) => write!(formatter, "{error}"),
            Self::WebSocket(error) => write!(formatter, "daemon WebSocket failed: {error}"),
            Self::Schema(error) => write!(formatter, "daemon RPC schema failed: {error}"),
            Self::Registry(error) => write!(formatter, "daemon RPC registry failed: {error}"),
            Self::Connection(error) => write!(formatter, "daemon RPC connection failed: {error}"),
            Self::Watch(error) => write!(formatter, "daemon file watch failed: {error}"),
            Self::Blob(error) => write!(formatter, "daemon BlobStore failed: {error}"),
            Self::Workspace(error) => write!(formatter, "daemon workspace failed: {error}"),
            Self::Thread => write!(formatter, "daemon thread panicked"),
            Self::Shutdown { failures } => {
                write!(formatter, "daemon shutdown failed")?;
                for failure in failures {
                    write!(formatter, "; {failure}")?;
                }

                Ok(())
            }
        }
    }
}

impl error::Error for DaemonError {
    /// Return the underlying daemon failure when present.
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Endpoint(error) => Some(error),
            Self::WebSocket(error) => Some(error),
            Self::Schema(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::Connection(error) => Some(error),
            Self::Watch(error) => Some(error),
            Self::Blob(error) => Some(error),
            Self::Workspace(error) => Some(error),
            Self::Thread => None,
            Self::Shutdown { failures } => failures
                .first()
                .map(|failure| failure as &(dyn error::Error + 'static)),
        }
    }
}

impl From<DaemonEndpointError> for DaemonError {
    /// Convert one endpoint failure.
    fn from(error: DaemonEndpointError) -> Self {
        Self::Endpoint(error)
    }
}

impl From<IpcError> for DaemonError {
    /// Convert one local transport failure.
    fn from(error: IpcError) -> Self {
        Self::Endpoint(error.into())
    }
}

impl From<TransportError> for DaemonError {
    /// Convert one established transport failure.
    fn from(error: TransportError) -> Self {
        Self::Connection(ConnectionError::from(error).into())
    }
}

impl From<WebSocketError> for DaemonError {
    /// Convert one WebSocket listener failure.
    fn from(error: WebSocketError) -> Self {
        Self::WebSocket(error)
    }
}

impl From<ServiceSchemaError> for DaemonError {
    /// Convert one service declaration failure.
    fn from(error: ServiceSchemaError) -> Self {
        Self::Schema(error)
    }
}

impl From<RegistryError> for DaemonError {
    /// Convert one service registration failure.
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<ServerError> for DaemonError {
    /// Convert one RPC connection failure.
    fn from(error: ServerError) -> Self {
        Self::Connection(error)
    }
}

impl From<FileWatchError> for DaemonError {
    /// Convert one physical file watch failure.
    fn from(error: FileWatchError) -> Self {
        Self::Watch(error)
    }
}

impl From<BlobStoreError> for DaemonError {
    /// Convert one immutable Blob storage failure.
    fn from(error: BlobStoreError) -> Self {
        Self::Blob(error)
    }
}

impl From<workspace::Error> for DaemonError {
    /// Convert one workspace initialization failure.
    fn from(error: workspace::Error) -> Self {
        Self::Workspace(error)
    }
}
