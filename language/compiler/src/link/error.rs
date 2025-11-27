use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskDependency};

/// Error when linking something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkError {
    /// Wait for task dependency.
    Yield { wait: CompileTaskDependency },
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

impl LinkError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::MissingTarget { .. } => 1,
            Self::UnresolvedSymbol { .. } => 2,
            Self::ConflictingSymbol { .. } => 3,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Yield { wait } => wait.first_node(),
            Self::MissingTarget { node, .. } => Some(*node),
            Self::UnresolvedSymbol { node, .. } => Some(*node),
            Self::ConflictingSymbol { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "unresolved dependency".to_string(),
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
                &format!("E{}{:03}", CompilePhase::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type LinkResult<T> = Result<T, LinkError>;

impl From<LinkError> for CompileError {
    #[inline]
    fn from(error: LinkError) -> Self {
        CompileError::Link(error)
    }
}
