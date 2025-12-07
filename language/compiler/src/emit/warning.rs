use std::path::PathBuf;

use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

use crate::TaskPhase;

/// Warning when emitting output.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EmitWarning {
    /// Overwriting an existing file.
    OverwritingFile {
        node: GlobalNodeIdAny,
        path: PathBuf,
    },
    /// Output file is unchanged from previous emit.
    FileUnchanged {
        node: GlobalNodeIdAny,
        path: PathBuf,
    },
}

impl EmitWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::OverwritingFile { .. } => 1,
            Self::FileUnchanged { .. } => 2,
        }
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::OverwritingFile { node, .. } => *node,
            Self::FileUnchanged { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::OverwritingFile { path, .. } => {
                format!("overwriting existing file: {}", path.display())
            }
            Self::FileUnchanged { path, .. } => {
                format!("file unchanged: {}", path.display())
            }
        }
    }
}

impl std::fmt::Display for EmitWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmitWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Emit.letter(), self.sub_code()),
            )
            .finish()
    }
}
