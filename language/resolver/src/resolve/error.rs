use std::io;
use std::path::PathBuf;

/// Resolution error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolverError {
    /// Path explicitly ignored.
    Ignored {
        /// The ignored path.
        path: PathBuf,
    },

    /// Module not found.
    NotFound {
        /// The original import specifier.
        specifier: String,
    },

    /// Matched alias value not found.
    MatchedAliasNotFound {
        /// The original import specifier.
        specifier: String,
        /// The alias key that matched.
        alias_key: String,
    },

    /// Destack not found.
    DestackNotFound {
        /// The missing config path.
        path: PathBuf,
    },

    /// Invalid Destack config.
    DestackInvalid {
        /// The invalid config path.
        path: PathBuf,
    },

    /// Circular Destack config extends.
    DestackCircular {
        /// The cycle of config paths.
        paths: Vec<PathBuf>,
    },

    /// IO error.
    IoError {
        /// The path involved in the IO failure.
        path: PathBuf,
        /// The underlying IO error kind.
        kind: io::ErrorKind,
    },

    /// Repository backed resolution failed.
    RepositoryError {
        /// The repository operation context path.
        path: PathBuf,
        /// The underlying repository error message.
        message: String,
    },

    /// File base resolution received a non-file path.
    ExpectedFilePath {
        /// The path that was expected to be a file.
        path: PathBuf,
    },

    /// Directory base resolution received a non-directory path.
    ExpectedDirectoryPath {
        /// The path that was expected to be a directory.
        path: PathBuf,
    },

    /// Path contains an unsupported platform construct.
    UnsupportedPath {
        /// The unsupported path.
        path: PathBuf,
    },

    /// Path specifier cannot be parsed.
    InvalidSpecifier {
        /// The original specifier text.
        specifier: String,
        /// The optional parse error detail.
        message: Option<String>,
    },

    /// Recursive or too deep dependency.
    RecursiveDependency {
        /// The recursion depth at failure.
        depth: u8,
    },
}

/// Resolver result.
pub type ResolverResult<T> = Result<T, ResolverError>;

impl ResolverError {
    /// Check if the error is the ignored path error.
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored { .. })
    }

    /// Get the numeric subcode of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Ignored { .. } => 1,
            Self::NotFound { .. } => 2,
            Self::MatchedAliasNotFound { .. } => 3,
            Self::DestackNotFound { .. } => 21,
            Self::DestackInvalid { .. } => 22,
            Self::DestackCircular { .. } => 23,
            Self::IoError { .. } => 7,
            Self::ExpectedFilePath { .. } => 9,
            Self::ExpectedDirectoryPath { .. } => 12,
            Self::UnsupportedPath { .. } => 8,
            Self::InvalidSpecifier { .. } => 11,
            Self::RecursiveDependency { .. } => 20,
            Self::RepositoryError { .. } => 26,
        }
    }

    /// Get the message string for this error.
    pub fn message(&self) -> String {
        match self {
            Self::Ignored { path } => format!("path is ignored {path:?}"),
            Self::NotFound { specifier } => format!("module '{specifier}' not found"),
            Self::MatchedAliasNotFound {
                specifier,
                alias_key,
            } => format!("module '{specifier}' not found for matched alias key '{alias_key}'"),
            Self::DestackNotFound { path } => format!("destack config '{path:?}' not found"),
            Self::DestackInvalid { path } => format!("invalid destack config '{path:?}'"),
            Self::DestackCircular { paths } => {
                format!("destack config extends configs circularly: {paths:?}")
            }
            Self::IoError { path, kind } => format!("IO error at {path:?}: {kind}"),
            Self::RepositoryError { path, message } => {
                format!("repository resolution error at {path:?}: {message}")
            }
            Self::ExpectedFilePath { path } => {
                format!("expected a file path for file base resolution, got {path:?}")
            }
            Self::ExpectedDirectoryPath { path } => {
                format!("expected a directory path for directory base resolution, got {path:?}")
            }
            Self::UnsupportedPath { path } => {
                format!("path {path:?} contains unsupported construct")
            }
            Self::InvalidSpecifier { specifier, message } => {
                if let Some(message) = message {
                    format!("invalid specifier '{specifier:?}': {message}")
                } else {
                    format!("invalid specifier '{specifier:?}'")
                }
            }
            Self::RecursiveDependency { depth } => {
                format!("recursion while resolving at depth {depth}")
            }
        }
    }

    /// Return true when this error means one candidate missed and resolution can try the next.
    pub const fn is_alternative_candidate_miss(&self) -> bool {
        matches!(
            self,
            Self::NotFound { .. } | Self::MatchedAliasNotFound { .. }
        )
    }
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ResolverError {}
