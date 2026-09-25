use std::path::{Path, PathBuf};
use std::{error, fmt};

/// One persistent artifact cache failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactCacheError {
    /// One persistent record failed validation or decoding.
    Record {
        /// The invalid record path.
        path: PathBuf,
        /// The exact record error.
        source: Box<ArtifactCacheError>,
    },
    /// One cache file operation failed.
    FileSystem {
        /// The failed operation.
        operation: &'static str,
        /// The affected cache path.
        path: PathBuf,
        /// The host error.
        message: String,
    },
    /// One cache file failed to encode or decode.
    Codec(tspp_serde::Error),
    /// One cache record violates its persistent invariants.
    Invalid(String),
    /// One running process is using this build cache.
    BuildInUse {
        /// The active build cache directory.
        path: PathBuf,
    },
    /// Artifact cache logic failed internally.
    Internal(String),
}

impl fmt::Display for ArtifactCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Record { path, source } => write!(
                formatter,
                "artifact cache record '{}' failed: {source}",
                path.display()
            ),
            Self::FileSystem {
                operation,
                path,
                message,
            } => write!(
                formatter,
                "artifact cache {operation} failed for '{}': {message}",
                path.display()
            ),
            Self::Codec(error) => write!(formatter, "artifact cache codec failed: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid artifact cache: {message}"),
            Self::BuildInUse { path } => write!(
                formatter,
                "artifact cache build '{}' is in use",
                path.display()
            ),
            Self::Internal(message) => {
                write!(formatter, "artifact cache internal error: {message}")
            }
        }
    }
}

impl error::Error for ArtifactCacheError {}

impl ArtifactCacheError {
    /// Attach one persistent record path to this error.
    pub(crate) fn record(self, path: &Path) -> Self {
        Self::Record {
            path: path.to_path_buf(),
            source: Box::new(self),
        }
    }

    /// Return whether this error identifies one invalid persistent record.
    pub fn is_invalid_record(&self) -> bool {
        matches!(
            self,
            Self::Record { source, .. }
                if matches!(source.as_ref(), Self::Codec(_) | Self::Invalid(_))
        )
    }

    /// Return the persistent record path attached to this error.
    pub(crate) fn record_path(&self) -> Option<&Path> {
        match self {
            Self::Record { path, .. } => Some(path),
            Self::FileSystem { .. }
            | Self::Codec(_)
            | Self::Invalid(_)
            | Self::BuildInUse { .. }
            | Self::Internal(_) => None,
        }
    }
}

impl From<tspp_serde::Error> for ArtifactCacheError {
    fn from(error: tspp_serde::Error) -> Self {
        Self::Codec(error)
    }
}
