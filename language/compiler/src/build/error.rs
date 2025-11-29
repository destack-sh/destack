use destack_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, TaskDependency, TaskError};

/// Error when building something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BuildError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
    /// Target is not available.
    TargetNotAvailable { node: GlobalNodeIdAny },
    /// Unsupported target triple / architecture / ABI.
    UnsupportedTarget {
        node: GlobalNodeIdAny,
        target: String,
    },
    /// Missing entry point (main/_start) when required.
    MissingEntryPoint { node: GlobalNodeIdAny },
    /// Unresolved external symbol / missing library at link time.
    UnresolvedSymbol { node: GlobalNodeIdAny },
    /// Duplicate symbols with incompatible declarations.
    DuplicateSymbol { node: GlobalNodeIdAny },
    /// Incompatible object formats or library formats.
    IncompatibleFormat {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Exceeding target limitations (too large TLS, section > size limit, etc.).
    TargetLimitExceeded { node: GlobalNodeIdAny },
    /// Failure to write output file (permissions, disk full, etc.).
    WriteFailure {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Required runtime component missing (no standard lib for this target).
    MissingRuntime {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
}

impl TryFrom<BuildError> for TaskDependency {
    type Error = BuildError;

    fn try_from(error: BuildError) -> Result<Self, Self::Error> {
        match error {
            BuildError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

pub type BuildResult<T> = Result<T, BuildError>;

impl From<BuildError> for TaskError {
    #[inline]
    fn from(error: BuildError) -> Self {
        TaskError::Build(error)
    }
}

impl BuildError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::YieldFailed { .. } => 1,
            Self::TargetNotAvailable { .. } => 2,
            Self::UnsupportedTarget { .. } => 3,
            Self::MissingEntryPoint { .. } => 4,
            Self::UnresolvedSymbol { .. } => 5,
            Self::DuplicateSymbol { .. } => 6,
            Self::IncompatibleFormat { .. } => 7,
            Self::TargetLimitExceeded { .. } => 8,
            Self::WriteFailure { .. } => 9,
            Self::MissingRuntime { .. } => 10,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::TargetNotAvailable { node, .. } => *node,
            Self::UnsupportedTarget { node, .. } => *node,
            Self::MissingEntryPoint { node, .. } => *node,
            Self::UnresolvedSymbol { node, .. } => *node,
            Self::DuplicateSymbol { node, .. } => *node,
            Self::IncompatibleFormat { node, .. } => *node,
            Self::TargetLimitExceeded { node, .. } => *node,
            Self::WriteFailure { node, .. } => *node,
            Self::MissingRuntime { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
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
                &format!("E{}{:03}", Phase::Build.letter(), self.sub_code()),
            )
            .finish()
    }
}
