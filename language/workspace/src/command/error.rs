use std::error::Error;
use std::fmt;

/// Error returned by workspace command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError {
    /// Error kind.
    pub kind: CommandErrorKind,
    /// Error message.
    pub message: String,
}

/// Kind of workspace command error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandErrorKind {
    /// Invalid command input.
    InvalidInput,
    /// Command configuration failure.
    Config,
    /// Module or target resolution failure.
    Resolve,
    /// Compiler pipeline failure.
    Compiler,
    /// Runtime execution failure.
    Runtime,
    /// Source publication failure.
    Source,
    /// Payload serialization or decoding failure.
    Payload,
    /// Internal command failure.
    Internal,
}

impl CommandError {
    /// Create an invalid input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::InvalidInput, message)
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Config, message)
    }

    /// Create a resolution error.
    pub fn resolve(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Resolve, message)
    }

    /// Create a compiler error.
    pub fn compiler(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Compiler, message)
    }

    /// Create a runtime error.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Runtime, message)
    }

    /// Create a source publication error.
    pub fn source(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Source, message)
    }

    /// Create a payload error.
    pub fn payload(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Payload, message)
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Internal, message)
    }

    /// Return the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Create a command error with kind and message.
    fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandError {
    /// Format the command error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            CommandErrorKind::InvalidInput => {
                write!(formatter, "invalid input: {}", self.message)
            }
            CommandErrorKind::Config => write!(formatter, "config error: {}", self.message),
            CommandErrorKind::Resolve => write!(formatter, "resolve error: {}", self.message),
            CommandErrorKind::Compiler => write!(formatter, "compiler error: {}", self.message),
            CommandErrorKind::Runtime => write!(formatter, "runtime error: {}", self.message),
            CommandErrorKind::Source => write!(formatter, "source error: {}", self.message),
            CommandErrorKind::Payload => write!(formatter, "payload error: {}", self.message),
            CommandErrorKind::Internal => write!(formatter, "internal error: {}", self.message),
        }
    }
}

impl Error for CommandError {}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::internal(message)
    }
}

/// Result alias for workspace command execution.
pub type CommandResult<T> = Result<T, CommandError>;
