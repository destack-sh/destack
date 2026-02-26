use std::error::Error;
use std::fmt;

/// Error returned by daemon command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonCommandError {
    /// Invalid command input.
    InvalidInput(String),
    /// Command configuration failure.
    Config(String),
    /// Module or target resolution failure.
    Resolve(String),
    /// Compiler pipeline failure.
    Compiler(String),
    /// Runtime execution failure.
    Runtime(String),
    /// Payload serialization or decoding failure.
    Payload(String),
    /// Internal command failure.
    Internal(String),
}

impl DaemonCommandError {
    /// Create an invalid input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a resolution error.
    pub fn resolve(message: impl Into<String>) -> Self {
        Self::Resolve(message.into())
    }

    /// Create a compiler error.
    pub fn compiler(message: impl Into<String>) -> Self {
        Self::Compiler(message.into())
    }

    /// Create a runtime error.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }

    /// Create a payload error.
    pub fn payload(message: impl Into<String>) -> Self {
        Self::Payload(message.into())
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Return the error message.
    pub fn message(&self) -> &str {
        match self {
            Self::InvalidInput(message)
            | Self::Config(message)
            | Self::Resolve(message)
            | Self::Compiler(message)
            | Self::Runtime(message)
            | Self::Payload(message)
            | Self::Internal(message) => message,
        }
    }
}

impl fmt::Display for DaemonCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "invalid input: {message}"),
            Self::Config(message) => write!(formatter, "config error: {message}"),
            Self::Resolve(message) => write!(formatter, "resolve error: {message}"),
            Self::Compiler(message) => write!(formatter, "compiler error: {message}"),
            Self::Runtime(message) => write!(formatter, "runtime error: {message}"),
            Self::Payload(message) => write!(formatter, "payload error: {message}"),
            Self::Internal(message) => write!(formatter, "internal error: {message}"),
        }
    }
}

impl Error for DaemonCommandError {}

impl From<String> for DaemonCommandError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

impl From<&str> for DaemonCommandError {
    fn from(message: &str) -> Self {
        Self::internal(message)
    }
}

/// Result alias for daemon command execution.
pub type CommandResult<T> = Result<T, DaemonCommandError>;
