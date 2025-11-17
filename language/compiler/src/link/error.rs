use dyst_dir::{NodeIdAny, Session};

use crate::{CompileError, CompilerStage};

/// Error when linking something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum LinkError {
    /// Optimization is impossible for this node.
    OptimizationImpossible { node: NodeIdAny } = 1,
}

impl LinkError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::OptimizationImpossible { .. } => 1,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::OptimizationImpossible { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::OptimizationImpossible { .. } => "optimization is impossible".to_string(),
        }
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkError")
            .field(
                "code",
                &format!("{}E{:03}", CompilerStage::Link.letter(), self.sub_code()),
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
