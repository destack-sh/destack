use destack_compiler_macros::DefineError;
use destack_mir as mir;
use destack_source::PackageId;
use destack_workspace::{Program, TargetId};

use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};

/// Errors during the optimize phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Optimize)]
pub enum OptimizeError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
    /// Wait for task dependency.
    #[error(code = "EO000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EO001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    // -------------------------------------------------------------------------
    // 1xx: Target / setup
    // -------------------------------------------------------------------------
    /// Invalid target configuration for optimization.
    #[error(code = "EO110", message = "invalid target {target}: {message}")]
    InvalidTarget {
        package: PackageId,
        target: TargetId,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 1xx: Move / ownership errors
    // -------------------------------------------------------------------------
    /// Value used after ownership was transferred.
    #[error(code = "EO100", message = "use of moved value")]
    UseAfterMove {
        node: mir::AnchoredGlobalNodeId,
        moved_at: mir::AnchoredGlobalNodeId,
    },

    /// Value moved multiple times.
    #[error(code = "EO101", message = "value moved twice")]
    DoubleMove {
        node: mir::AnchoredGlobalNodeId,
        first_move: mir::AnchoredGlobalNodeId,
    },

    /// Cannot move a value that is currently borrowed.
    #[error(code = "EO102", message = "cannot move while borrowed")]
    MoveOfBorrowedValue {
        node: mir::AnchoredGlobalNodeId,
        borrowed_at: mir::AnchoredGlobalNodeId,
    },

    /// Using a partially-moved aggregate (field was moved, then whole struct used).
    #[error(code = "EO103", message = "use of partially moved value")]
    PartialMove {
        node: mir::AnchoredGlobalNodeId,
        moved_at: mir::AnchoredGlobalNodeId,
    },

    /// Value may have been moved (moved on some control flow paths but not others).
    #[error(code = "EO104", message = "value may have been moved")]
    MaybeUseAfterMove {
        node: mir::AnchoredGlobalNodeId,
        moved_at: mir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Borrow errors
    // -------------------------------------------------------------------------
    /// Mutable borrow conflicts with existing borrow.
    #[error(code = "EO200", message = "cannot borrow as mutable: already borrowed")]
    ConflictingBorrow {
        node: mir::AnchoredGlobalNodeId,
        existing_borrow: mir::AnchoredGlobalNodeId,
        existing_is_mutable: bool,
    },

    /// Reference used after the borrowed value was mutated through another path.
    #[error(code = "EO201", message = "borrow invalidated by mutation")]
    InvalidatedReference {
        node: mir::AnchoredGlobalNodeId,
        invalidated_by: mir::AnchoredGlobalNodeId,
    },

    /// Borrow outlives the value it borrows from.
    #[error(code = "EO202", message = "borrow escapes scope")]
    BorrowEscapesScope {
        node: mir::AnchoredGlobalNodeId,
        escapes_at: mir::AnchoredGlobalNodeId,
    },

    /// Attempting to borrow a value that was already moved.
    #[error(code = "EO203", message = "cannot borrow: value was moved")]
    BorrowOfMovedValue {
        node: mir::AnchoredGlobalNodeId,
        moved_at: mir::AnchoredGlobalNodeId,
    },

    /// Assigning to a local variable while it is borrowed.
    #[error(code = "EO204", message = "cannot assign to local: value is borrowed")]
    LocalSetWhileBorrowed {
        node: mir::AnchoredGlobalNodeId,
        borrowed_at: mir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 3xx: Lifetime errors
    // -------------------------------------------------------------------------
    /// Returning a reference to a local variable.
    #[error(code = "EO300", message = "cannot return reference to local")]
    ReturnReferenceToLocal { node: mir::AnchoredGlobalNodeId },

    /// Reference to local stored in longer-lived location.
    #[error(code = "EO301", message = "reference to local escapes function")]
    LocalReferenceEscapes { node: mir::AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 4xx: Drop errors
    // -------------------------------------------------------------------------
    /// Dropping a value while it is borrowed.
    #[error(code = "EO400", message = "cannot drop: value is borrowed")]
    DropWhileBorrowed {
        node: mir::AnchoredGlobalNodeId,
        borrowed_at: mir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported MIR construct encountered during optimization.
    #[error(code = "EO900", message = "unsupported MIR construct")]
    UnsupportedConstruct { node: mir::AnchoredGlobalNodeId },

    /// Internal optimization error.
    #[error(code = "EO901", message = "internal optimization error: {message}")]
    InternalError {
        node: mir::AnchoredGlobalNodeId,
        message: String,
    },
}
