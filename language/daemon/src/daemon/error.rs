use std::path::PathBuf;
use std::{error, fmt};

use tspp_core::BlobStoreError;
use tspp_rpc::{
    ConnectionError, IpcError, RegistryError, ServerError, ServiceSchemaError, TransportError,
    WebSocketError,
};
use tspp_source::FileWatchError;
use tspp_workspace as workspace;

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
    Watch(WorkspaceWatchError),
    /// Immutable Blob storage failed.
    Blob(BlobStoreError),
    /// Workspace initialization failed.
    Workspace(Box<workspace::Error>),
    /// A daemon thread panicked.
    Thread,
    /// Multiple failures occurred while terminating daemon owners.
    Shutdown {
        /// Serving and cleanup failures in observation order.
        failures: Vec<DaemonError>,
    },
}

/// Failure while observing one daemon workspace.
#[derive(Debug)]
pub enum WorkspaceWatchError {
    /// The operating-system watcher could not observe the workspace.
    FileWatch {
        /// Workspace root that could not be observed.
        root: PathBuf,
        /// Host watcher failure.
        source: FileWatchError,
    },
    /// The operating-system watcher stopped producing events.
    Disconnected {
        /// Workspace root whose watcher stopped.
        root: PathBuf,
    },
    /// Authoritative physical state could not be reloaded.
    Reload {
        /// Workspace root that could not be reloaded.
        root: PathBuf,
        /// Workspace reload failure.
        source: Box<workspace::Error>,
    },
    /// Authoritative reload failed after a host watcher error.
    FileWatchReload {
        /// Workspace root that could not be reloaded.
        root: PathBuf,
        /// Host watcher failure that required recovery.
        source: FileWatchError,
        /// Workspace reload failure.
        reload: Box<workspace::Error>,
    },
    /// Authoritative reload failed after incremental reconciliation.
    Reconcile {
        /// Workspace root that could not be reconciled.
        root: PathBuf,
        /// Incremental reconciliation failure.
        source: Box<workspace::Error>,
        /// Authoritative reload failure.
        reload: Box<workspace::Error>,
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
            Self::Watch(error) => write!(formatter, "daemon workspace watch failed: {error}"),
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
            Self::Workspace(error) => Some(error.as_ref()),
            Self::Thread => None,
            Self::Shutdown { failures } => failures
                .first()
                .map(|failure| failure as &(dyn error::Error + 'static)),
        }
    }
}

impl fmt::Display for WorkspaceWatchError {
    /// Format this workspace observation failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileWatch { root, source } => {
                write!(
                    formatter,
                    "host file watch failed for {}: {source}",
                    root.display()
                )
            }
            Self::Disconnected { root } => {
                write!(
                    formatter,
                    "host file watch stopped unexpectedly for {}",
                    root.display()
                )
            }
            Self::Reload { root, source } => {
                write!(
                    formatter,
                    "workspace reload failed for {}: {source}",
                    root.display()
                )
            }
            Self::FileWatchReload {
                root,
                source,
                reload,
            } => {
                write!(
                    formatter,
                    "host file watch failed for {}: {source}; workspace reload failed: {reload}",
                    root.display()
                )
            }
            Self::Reconcile {
                root,
                source,
                reload,
            } => {
                write!(
                    formatter,
                    "workspace reconciliation failed for {}: {source}; \
                     workspace reload failed: {reload}",
                    root.display()
                )
            }
        }
    }
}

impl error::Error for WorkspaceWatchError {
    /// Return the failure that initiated this terminal observation error.
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::FileWatch { source, .. } => Some(source),
            Self::Disconnected { .. } => None,
            Self::Reload { source, .. } => Some(source.as_ref()),
            Self::FileWatchReload { source, .. } => Some(source),
            Self::Reconcile { source, .. } => Some(source.as_ref()),
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

impl From<WorkspaceWatchError> for DaemonError {
    /// Convert one workspace observation failure.
    fn from(error: WorkspaceWatchError) -> Self {
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
        Self::Workspace(Box::new(error))
    }
}
