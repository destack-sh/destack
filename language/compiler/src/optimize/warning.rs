use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the optimize phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Optimize)]
pub enum OptimizeWarning {
    /// Inscrutable type for an expression.
    #[warning(code = "WO001", message = "inscrutable type")]
    InscrutableType { node: GlobalNodeIdAny },

    /// Hint ignored.
    #[warning(code = "WO002", message = "ignored hint")]
    IgnoredHint {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },

    /// Optimization skipped.
    #[warning(code = "WO003", message = "optimization skipped")]
    SkippedOptimization {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
}
