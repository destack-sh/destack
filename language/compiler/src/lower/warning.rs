use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_dir as dir;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Lower)]
pub enum LowerWarning {
    // -------------------------------------------------------------------------
    // 1xx: Type warnings
    // -------------------------------------------------------------------------
    /// Complex type in target language.
    #[diagnostic(code = "WL100", message = "complex type in target")]
    ComplexType {
        /// Point at the complex type source.
        anchor: DiagnosticAnchor,
    },

    /// Type coercion may lose precision.
    #[diagnostic(code = "WL101", message = "type coercion may lose precision")]
    PrecisionLoss {
        /// Point at the cast/coercion node.
        anchor: DiagnosticAnchor,
        /// Source type.
        from_ty: dir::GlobalTypeId,
        /// Target type.
        to_ty: dir::GlobalTypeId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Performance warnings
    // -------------------------------------------------------------------------
    /// Large aggregate copy (struct/array).
    #[diagnostic(code = "WL200", message = "large aggregate copy ({size_bytes} bytes)")]
    LargeAggregateCopy {
        /// Point at the copy expression.
        anchor: DiagnosticAnchor,
        /// Size in bytes of the aggregate being copied.
        size_bytes: u64,
    },

    // -------------------------------------------------------------------------
    // 3xx: Code quality warnings
    // -------------------------------------------------------------------------
    /// Unreachable code after terminator.
    #[diagnostic(
        code = "WL300",
        message = "unreachable code after return/break/continue"
    )]
    UnreachableCode {
        /// Point at the unreachable statement.
        anchor: DiagnosticAnchor,
    },

    /// Empty block with no effect.
    #[diagnostic(code = "WL301", message = "empty block has no effect")]
    EmptyBlock {
        /// Point at the empty block.
        anchor: DiagnosticAnchor,
    },
}
