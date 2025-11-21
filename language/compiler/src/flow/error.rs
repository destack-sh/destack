use dyst_dir::{GlobalNodeIdAny, Session};

use crate::{CompileError, CompilePhase};

/// Error when flowing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum FlowError {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl FlowError {
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

impl std::fmt::Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowError")
            .field(
                "code",
                &format!("{}E{:03}", CompilePhase::Flow.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<FlowError> for CompileError {
    #[inline]
    fn from(error: FlowError) -> Self {
        CompileError::Flow(error)
    }
}

pub type FlowResult<T> = Result<T, FlowError>;
