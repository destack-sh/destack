use crate::{
    BuildRequirementError, BuildRequirementSet, DiagnosticAnchor, DiagnosticDefinition, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir as dir;
use destack_workspace::Program;
use dir::AnchoredGlobalNodeId;

/// Errors during the elaborate phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Elaborate)]
pub enum ElaborateError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for build requirement.
    #[error(code = "EE000", r#yield)]
    Yield { requirement: BuildRequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EE001", yield_failed)]
    UnsatisfiedRequirement { requirement: BuildRequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EE002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Configuration
    // -------------------------------------------------------------------------
    /// Implicit collection conversions are disabled by configuration.
    #[error(
        code = "EE100",
        message = "implicit collection conversions are disabled"
    )]
    ImplicitCollectionConversion { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "EE900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
