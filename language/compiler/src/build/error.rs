use dyst_dir::{NodeIdAny, Session};

use crate::{CompileError, CompilerStage};

/// Error when building something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BuildError {
    /// Target is not available.
    TargetNotAvailable { node: NodeIdAny } = 1,
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
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::TargetNotAvailable { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::TargetNotAvailable { .. } => "target is not available".to_string(),
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildError")
            .field(
                "code",
                &format!("{}E{:03}", CompilerStage::Build.letter(), self.sub_code()),
            )
            .finish()
    }
}
