use destack_artifact::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_mir as mir;
use destack_source::{ModuleId, PackageId, TargetId};

use crate::CompilerError;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Lower)]
pub enum LowerError {
    /// One invalid target configuration.
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

    /// One construct the MIR lowering rejects.
    #[diagnostic(
        id = "unsupported-lower-construct",
        message = "unsupported construct: {construct}"
    )]
    Unsupported {
        /// The module containing the unsupported construct.
        anchor: DiagnosticAnchor,
        /// The unsupported construct.
        construct: String,
    },
}

impl From<(ModuleId, mir::LayoutError)> for CompilerError {
    /// Convert one MIR layout error into a compiler error.
    fn from((module, error): (ModuleId, mir::LayoutError)) -> Self {
        match error {
            mir::LayoutError::Missing { ty } => Self::Internal {
                message: format!("missing required MIR layout for {ty:?}"),
            },
            mir::LayoutError::Unresolved(ty) => Self::Internal {
                message: format!("unresolved layout for {ty:?}"),
            },
            mir::LayoutError::Unsupported { construct } => LowerError::Unsupported {
                anchor: module.into(),
                construct,
            }
            .into(),
            mir::LayoutError::InvalidDiscriminant { constant } => Self::Internal {
                message: format!("a non-scalar variant tag {constant}"),
            },
            mir::LayoutError::InvalidNiche { offset } => Self::Internal {
                message: format!("a missing variant niche at byte offset {offset}"),
            },
        }
    }
}
