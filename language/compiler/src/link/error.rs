use destack_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, TaskDependency, TaskError};

/// Error when linking something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
    /// Missing target for a symbol.
    MissingTarget {
        node: GlobalNodeIdAny,
        symbol: String,
    },
    /// Unresolved external symbol.
    UnresolvedSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },
    /// Duplicate symbols with incompatible declarations.
    ConflictingSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },
}

impl TryFrom<LinkError> for TaskDependency {
    type Error = LinkError;

    fn try_from(error: LinkError) -> Result<Self, Self::Error> {
        match error {
            LinkError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl LinkError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::YieldFailed { .. } => 1,
            Self::MissingTarget { .. } => 2,
            Self::UnresolvedSymbol { .. } => 3,
            Self::ConflictingSymbol { .. } => 4,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::MissingTarget { node, .. } => *node,
            Self::UnresolvedSymbol { node, .. } => *node,
            Self::ConflictingSymbol { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
            Self::MissingTarget { .. } => "missing target".to_string(),
            Self::UnresolvedSymbol { .. } => "unresolved symbol".to_string(),
            Self::ConflictingSymbol { .. } => "conflicting symbol".to_string(),
        }
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkError")
            .field(
                "code",
                &format!("E{}{:03}", Phase::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type LinkResult<T> = Result<T, LinkError>;

impl From<LinkError> for TaskError {
    #[inline]
    fn from(error: LinkError) -> Self {
        TaskError::Link(error)
    }
}
