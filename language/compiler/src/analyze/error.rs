use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase};

/// Error when analyzeing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum AnalyzeError {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl AnalyzeError {
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
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeError")
            .field(
                "code",
                &format!("{}E{:03}", CompilePhase::Analyze.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<AnalyzeError> for CompileError {
    #[inline]
    fn from(error: AnalyzeError) -> Self {
        CompileError::Analyze(error)
    }
}

pub type AnalyzeResult<T> = Result<T, AnalyzeError>;
