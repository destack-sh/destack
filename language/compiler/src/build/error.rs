use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileError, CompilePhase};

/// Error when building something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BuildError {
    /// Target is not available.
    TargetNotAvailable { node: LocalNodeIdAny },
    /// Unsupported target triple / architecture / ABI.
    UnsupportedTarget {
        node: LocalNodeIdAny,
        target: String,
    },
    /// Missing entry point (main/_start) when required.
    MissingEntryPoint { node: LocalNodeIdAny },
    /// Unresolved external symbol / missing library at link time.
    UnresolvedSymbol { node: LocalNodeIdAny },
    /// Duplicate symbols with incompatible declarations.
    DuplicateSymbol { node: LocalNodeIdAny },
    /// Incompatible object formats or library formats.
    IncompatibleFormat {
        node: LocalNodeIdAny,
        message: Option<String>,
    },
    /// Exceeding target limitations (too large TLS, section > size limit, etc.).
    TargetLimitExceeded { node: LocalNodeIdAny },
    /// Failure to write output file (permissions, disk full, etc.).
    WriteFailure {
        node: LocalNodeIdAny,
        message: Option<String>,
    },
    /// Required runtime component missing (no standard lib for this target).
    MissingRuntime {
        node: LocalNodeIdAny,
        message: Option<String>,
    },
}

pub type BuildResult<T> = Result<T, BuildError>;

impl From<BuildError> for CompileError {
    #[inline]
    fn from(error: BuildError) -> Self {
        CompileError::Build(error)
    }
}

impl BuildError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::TargetNotAvailable { .. } => 1,
            Self::UnsupportedTarget { .. } => 2,
            Self::MissingEntryPoint { .. } => 3,
            Self::UnresolvedSymbol { .. } => 4,
            Self::DuplicateSymbol { .. } => 5,
            Self::IncompatibleFormat { .. } => 6,
            Self::TargetLimitExceeded { .. } => 7,
            Self::WriteFailure { .. } => 8,
            Self::MissingRuntime { .. } => 9,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::TargetNotAvailable { node, .. } => Some(*node),
            Self::UnsupportedTarget { node, .. } => Some(*node),
            Self::MissingEntryPoint { node, .. } => Some(*node),
            Self::UnresolvedSymbol { node, .. } => Some(*node),
            Self::DuplicateSymbol { node, .. } => Some(*node),
            Self::IncompatibleFormat { node, .. } => Some(*node),
            Self::TargetLimitExceeded { node, .. } => Some(*node),
            Self::WriteFailure { node, .. } => Some(*node),
            Self::MissingRuntime { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::TargetNotAvailable { .. } => "target is not available".to_string(),
            Self::UnsupportedTarget { .. } => "unsupported target".to_string(),
            Self::MissingEntryPoint { .. } => "missing entry point".to_string(),
            Self::UnresolvedSymbol { .. } => "unresolved symbol".to_string(),
            Self::DuplicateSymbol { .. } => "duplicate symbol".to_string(),
            Self::IncompatibleFormat { .. } => "incompatible format".to_string(),
            Self::TargetLimitExceeded { .. } => "target limit exceeded".to_string(),
            Self::WriteFailure { .. } => "write failure".to_string(),
            Self::MissingRuntime { .. } => "missing runtime".to_string(),
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildError")
            .field(
                "code",
                &format!("{}E{:03}", CompilePhase::Build.letter(), self.sub_code()),
            )
            .finish()
    }
}
