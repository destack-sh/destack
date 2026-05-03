use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{PackageId, TargetId};

/// Errors during the optimize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Optimize)]
pub enum OptimizeError {
    // -------------------------------------------------------------------------
    // 1xx: Target / setup
    // -------------------------------------------------------------------------
    /// Invalid target configuration for optimization.
    #[diagnostic(code = "EO110", message = "invalid target {target}: {message}")]
    InvalidTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 1xx: Move / ownership errors
    // -------------------------------------------------------------------------
    /// Value used after ownership was transferred.
    #[diagnostic(code = "EO100", message = "use of moved value")]
    UseAfterMove {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    /// Value moved multiple times.
    #[diagnostic(code = "EO101", message = "value moved twice")]
    DoubleMove {
        anchor: DiagnosticAnchor,
        first_move: DiagnosticAnchor,
    },

    /// Cannot move a value that is currently borrowed.
    #[diagnostic(code = "EO102", message = "cannot move while borrowed")]
    MoveOfBorrowedValue {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    /// Using a partially-moved aggregate (field was moved, then whole struct used).
    #[diagnostic(code = "EO103", message = "use of partially moved value")]
    PartialMove {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    /// Value may have been moved (moved on some control flow paths but not others).
    #[diagnostic(code = "EO104", message = "value may have been moved")]
    MaybeUseAfterMove {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 2xx: Borrow errors
    // -------------------------------------------------------------------------
    /// Mutable borrow conflicts with existing borrow.
    #[diagnostic(code = "EO200", message = "cannot borrow as mutable: already borrowed")]
    ConflictingBorrow {
        anchor: DiagnosticAnchor,
        existing_borrow: DiagnosticAnchor,
        existing_is_mutable: bool,
    },

    /// Reference used after the borrowed value was mutated through another path.
    #[diagnostic(code = "EO201", message = "borrow invalidated by mutation")]
    InvalidatedReference {
        anchor: DiagnosticAnchor,
        invalidated_by: DiagnosticAnchor,
    },

    /// Borrow outlives the value it borrows from.
    #[diagnostic(code = "EO202", message = "borrow escapes scope")]
    BorrowEscapesScope {
        anchor: DiagnosticAnchor,
        escapes_at: DiagnosticAnchor,
    },

    /// Attempting to borrow a value that was already moved.
    #[diagnostic(code = "EO203", message = "cannot borrow: value was moved")]
    BorrowOfMovedValue {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    /// Assigning to a local variable while it is borrowed.
    #[diagnostic(code = "EO204", message = "cannot assign to local: value is borrowed")]
    LocalSetWhileBorrowed {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 3xx: Lifetime errors
    // -------------------------------------------------------------------------
    /// Returning a reference to a local variable.
    #[diagnostic(code = "EO300", message = "cannot return reference to local")]
    ReturnReferenceToLocal { anchor: DiagnosticAnchor },

    /// Reference to local stored in longer-lived location.
    #[diagnostic(code = "EO301", message = "reference to local escapes function")]
    LocalReferenceEscapes { anchor: DiagnosticAnchor },

    /// Returned borrow does not match lifetime annotation.
    #[diagnostic(
        code = "EO302",
        message = "return borrows from {origin} not covered by lifetime annotation"
    )]
    LifetimeAnnotationMismatch {
        anchor: DiagnosticAnchor,
        origin: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: Drop errors
    // -------------------------------------------------------------------------
    /// Dropping a value while it is borrowed.
    #[diagnostic(code = "EO400", message = "cannot drop: value is borrowed")]
    DropWhileBorrowed {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 5xx: Metadata contract errors
    // -------------------------------------------------------------------------
    /// Required metadata is missing for the configured pipeline.
    #[diagnostic(code = "EO500", message = "missing required metadata: {message}")]
    MissingRequiredMetadata {
        anchor: DiagnosticAnchor,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported MIR construct encountered during optimization.
    #[diagnostic(code = "EO900", message = "unsupported MIR construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },

    /// Internal optimization error.
    #[diagnostic(code = "EO901", message = "internal optimization error: {message}")]
    InternalError {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
