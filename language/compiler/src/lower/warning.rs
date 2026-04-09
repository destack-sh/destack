use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir as dir;
use destack_workspace::Repository;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Lower)]
pub enum LowerWarning {
    // -------------------------------------------------------------------------
    // 1xx: Type warnings
    // -------------------------------------------------------------------------
    /// Complex type in target language.
    #[warning(code = "WM100", message = "complex type in target")]
    ComplexType {
        /// Point at the node that introduced a complex type.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Type coercion may lose precision.
    #[warning(
        code = "WM101",
        message = "type coercion may lose precision from {from_ty} to {to_ty}"
    )]
    PrecisionLoss {
        /// Point at the cast/coercion node.
        node: dir::AnchoredGlobalNodeId,
        /// Source type.
        from_ty: dir::GlobalTypeId,
        /// Target type.
        to_ty: dir::GlobalTypeId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Performance warnings
    // -------------------------------------------------------------------------
    /// Large aggregate copy (struct/array).
    #[warning(code = "WM200", message = "large aggregate copy ({size_bytes} bytes)")]
    LargeAggregateCopy {
        /// Point at the copy expression.
        node: dir::AnchoredGlobalNodeId,
        /// Size in bytes of the aggregate being copied.
        size_bytes: u64,
    },

    // -------------------------------------------------------------------------
    // 3xx: Code quality warnings
    // -------------------------------------------------------------------------
    /// Unreachable code after terminator.
    #[warning(
        code = "WM300",
        message = "unreachable code after return/break/continue"
    )]
    UnreachableCode {
        /// Point at the unreachable statement.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Empty block with no effect.
    #[warning(code = "WM301", message = "empty block has no effect")]
    EmptyBlock {
        /// Point at the empty block.
        node: dir::AnchoredGlobalNodeId,
    },
}
