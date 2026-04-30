use std::path::PathBuf;

use destack_compiler::ImportError;
use destack_service::LanguageServiceError;
use destack_source::{Diagnostic, FileId};

/// Diagnostics produced by daemon work.
#[derive(Debug, Clone, Default)]
pub struct DaemonDiagnostics {
    /// The diagnostic entries.
    pub diagnostics: Vec<Diagnostic>,
}

impl DaemonDiagnostics {
    /// Create a diagnostics wrapper.
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }
}

/// Errors produced while applying daemon updates.
#[derive(Debug)]
pub enum DaemonError {
    /// File system write failed.
    FileWrite {
        /// The file path that failed.
        path: PathBuf,
        /// The underlying write error.
        error: std::io::Error,
    },
    /// File path is not tracked in the program.
    FileMissing {
        /// The file path that failed.
        path: PathBuf,
    },
    /// File id is not tracked in the program.
    FileIdNotTracked {
        /// The file id that failed.
        file_id: FileId,
    },
    /// Module resolution failed.
    Resolve {
        /// The module path that failed.
        path: PathBuf,
        /// The underlying resolve error.
        error: Box<ImportError>,
    },
    /// Language service operation failed.
    Service {
        /// The typed language service error.
        error: LanguageServiceError,
    },
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format daemon errors
        match self {
            DaemonError::FileWrite { path, error } => {
                write!(f, "failed to write file {}: {}", path.display(), error)
            }
            DaemonError::FileMissing { path } => {
                write!(f, "file is not tracked: {}", path.display())
            }
            DaemonError::FileIdNotTracked { file_id } => {
                write!(f, "file id is not tracked: {file_id:?}")
            }
            DaemonError::Resolve { path, error } => {
                write!(f, "failed to resolve module {}: {error}", path.display())
            }
            DaemonError::Service { error } => {
                write!(f, "service error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<LanguageServiceError> for DaemonError {
    fn from(error: LanguageServiceError) -> Self {
        Self::Service { error }
    }
}
