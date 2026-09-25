use std::time::SystemTimeError;
use std::{fmt, io};

use tspp_rpc::{CallError, ConnectError, IpcError};

/// Failure to discover or change one daemon endpoint.
#[derive(Debug)]
pub enum DaemonEndpointError {
    /// Another process owns this endpoint.
    AlreadyRunning,
    /// Underlying filesystem failure.
    Io(io::Error),
    /// Endpoint metadata serialization failure.
    Serde(serde_json::Error),
    /// System clock failure.
    Time(SystemTimeError),
    /// Secure randomness failure.
    Random(getrandom::Error),
    /// Local transport failure.
    Ipc(IpcError),
    /// RPC connection failure.
    Connect(ConnectError),
    /// RPC invocation failure.
    Call(CallError),
}

impl fmt::Display for DaemonEndpointError {
    /// Format this daemon endpoint failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(formatter, "daemon is already running"),
            Self::Io(error) => write!(formatter, "daemon endpoint I/O failed: {error}"),
            Self::Serde(error) => write!(formatter, "daemon metadata failed: {error}"),
            Self::Time(error) => write!(formatter, "daemon clock failed: {error}"),
            Self::Random(error) => write!(formatter, "daemon randomness failed: {error}"),
            Self::Ipc(error) => write!(formatter, "daemon transport failed: {error}"),
            Self::Connect(error) => write!(formatter, "daemon connection failed: {error}"),
            Self::Call(error) => write!(formatter, "daemon call failed: {error}"),
        }
    }
}

impl std::error::Error for DaemonEndpointError {
    /// Return the underlying failure.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyRunning => None,
            Self::Io(error) => Some(error),
            Self::Serde(error) => Some(error),
            Self::Time(error) => Some(error),
            Self::Random(error) => Some(error),
            Self::Ipc(error) => Some(error),
            Self::Connect(error) => Some(error),
            Self::Call(error) => Some(error),
        }
    }
}

impl From<io::Error> for DaemonEndpointError {
    /// Convert one filesystem failure.
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DaemonEndpointError {
    /// Convert one metadata serialization failure.
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error)
    }
}

impl From<IpcError> for DaemonEndpointError {
    /// Convert one local transport failure.
    fn from(error: IpcError) -> Self {
        Self::Ipc(error)
    }
}

impl From<ConnectError> for DaemonEndpointError {
    /// Convert one RPC connection failure.
    fn from(error: ConnectError) -> Self {
        Self::Connect(error)
    }
}

impl From<CallError> for DaemonEndpointError {
    /// Convert one RPC invocation failure.
    fn from(error: CallError) -> Self {
        Self::Call(error)
    }
}
