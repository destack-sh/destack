use std::path::PathBuf;

use destack_workspace::Program;

use crate::{DiagnosticAnchor, TaskPhase};

/// Warning when emitting output.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EmitWarning {
    /// Overwriting an existing file.
    OverwritingFile { path: PathBuf },
    /// Output file is unchanged from previous emit.
    FileUnchanged { path: PathBuf },
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

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        // emit warnings are file-system related, not tied to source
        DiagnosticAnchor::Global
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
