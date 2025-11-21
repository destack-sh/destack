use dyst_dir::{GlobalNodeIdAny, Session};

use crate::{CompileError, CompilePhase};

/// Error when elaborateing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ElaborateError {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl ElaborateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 2,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ElaborateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElaborateError")
            .field(
                "code",
                &format!(
                    "{}E{:03}",
                    CompilePhase::Elaborate.letter(),
                    self.sub_code()
                ),
            )
            .finish()
    }
}

impl From<ElaborateError> for CompileError {
    #[inline]
    fn from(error: ElaborateError) -> Self {
        CompileError::Elaborate(error)
    }
}

pub type ElaborateResult<T> = Result<T, ElaborateError>;
