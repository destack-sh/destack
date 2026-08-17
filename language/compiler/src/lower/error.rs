use destack_artifact::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_mir as mir;
use destack_source::{ModuleId, PackageId, TargetId};

use crate::CompilerError;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Lower)]
pub enum LowerError {
    /// Invalid target configuration for lowering.
    #[diagnostic(
        id = "invalid-lower-target",
        message = "invalid target {target}: {message}"
    )]
    InvalidTarget {
        /// The package containing the invalid target.
        anchor: DiagnosticAnchor,
        /// The package being lowered.
        package: PackageId,
        /// The invalid target.
        target: TargetId,
        /// The invalid target detail.
        message: String,
    },

    /// Construct is not supported by MIR lowering.
    #[diagnostic(
        id = "unsupported-lower-construct",
        message = "MIR lowering does not support {construct}"
    )]
    Unsupported {
        /// Anchor the error to the unsupported construct.
        anchor: DiagnosticAnchor,
        /// The unsupported construct.
        construct: String,
    },
}

impl From<(ModuleId, mir::LayoutError)> for CompilerError {
    /// Convert one MIR layout error into a compiler error.
    fn from((module, error): (ModuleId, mir::LayoutError)) -> Self {
        match error {
            mir::LayoutError::Unsupported { construct } => LowerError::Unsupported {
                anchor: module.into(),
                construct,
            }
            .into(),
            mir::LayoutError::InvalidDiscriminant { constant } => Self::Internal {
                message: format!("variant case sealed a non-scalar tag {constant}"),
            },
            mir::LayoutError::InvalidNiche { offset } => Self::Internal {
                message: format!(
                    "variant niche at byte offset {offset} is absent from its representation"
                ),
            },
        }
    }
}
