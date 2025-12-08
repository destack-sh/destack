use crate::{DiagnosticAnchor, TaskPhase};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Warning when importing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportWarning {
    /// Huge file.
    OversizedFile { module: ModuleId, len: usize },
}

impl ImportWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::OversizedFile { .. } => 1,
        }
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::OversizedFile { module, .. } => DiagnosticAnchor::Module(*module),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::OversizedFile { len, .. } => format!("oversized file ({len} bytes)"),
        }
    }
}

impl std::fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Import.letter(), self.sub_code()),
            )
            .finish()
    }
}
