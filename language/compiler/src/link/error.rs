use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileError, CompilePhase};

/// Error when linking something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum LinkError {
    /// Missing target for a symbol.
    MissingTarget {
        node: LocalNodeIdAny,
        symbol: String,
    },
    /// Unresolved external symbol.
    UnresolvedSymbol {
        node: LocalNodeIdAny,
        symbol: String,
    },
    /// Duplicate symbols with incompatible declarations.
    ConflictingSymbol {
        node: LocalNodeIdAny,
        symbol: String,
    },
}

impl LinkError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingTarget { .. } => 1,
            Self::UnresolvedSymbol { .. } => 2,
            Self::ConflictingSymbol { .. } => 3,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::MissingTarget { node, .. } => Some(*node),
            Self::UnresolvedSymbol { node, .. } => Some(*node),
            Self::ConflictingSymbol { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
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
                &format!("{}E{:03}", CompilePhase::Link.letter(), self.sub_code()),
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
