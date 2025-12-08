use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskPhase, TaskWarning};

use destack_workspace::Program;

/// Warning during code generation.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum GenerateWarning {
    /// Imprecise type (loss of precision in codegen).
    ImpreciseType { node: GlobalNodeIdAny },
    /// Unexpected construct (recoverable).
    UnexpectedConstruct { node: GlobalNodeIdAny },
}

impl GenerateWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ImpreciseType { .. } => 1,
            Self::UnexpectedConstruct { .. } => 2,
        }
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::ImpreciseType { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnexpectedConstruct { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::ImpreciseType { .. } => "imprecise type".to_string(),
            Self::UnexpectedConstruct { .. } => "unexpected construct".to_string(),
        }
    }
}

impl std::fmt::Display for GenerateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Generate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<GenerateWarning> for TaskWarning {
    fn from(warning: GenerateWarning) -> Self {
        TaskWarning::Generate(warning)
    }
}
