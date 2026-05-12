use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Verify)]
pub enum VerifyError {
    /// Value used after ownership was transferred.
    #[diagnostic(code = "EV100", message = "use of moved value")]
    UseAfterMove {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    /// Value may have been moved on one control-flow path.
    #[diagnostic(code = "EV101", message = "value may have been moved")]
    MaybeUseAfterMove {
        anchor: DiagnosticAnchor,
        moved_at: DiagnosticAnchor,
    },

    /// A new borrow conflicts with an active borrow.
    #[diagnostic(code = "EV200", message = "borrow conflicts with active borrow")]
    ConflictingLoan {
        anchor: DiagnosticAnchor,
        existing_loan: DiagnosticAnchor,
    },

    /// Cannot change a place while an overlapping loan is live.
    #[diagnostic(code = "EV201", message = "cannot change borrowed place")]
    ChangeOfBorrowedPlace {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    /// Cannot write through readonly access.
    #[diagnostic(code = "EV202", message = "cannot write through readonly access")]
    ReadonlyWrite { anchor: DiagnosticAnchor },

    /// An escaping borrow is not covered by the required lifetime.
    #[diagnostic(
        code = "EV300",
        message = "borrow from {origin} does not live long enough"
    )]
    BorrowOutlivesOrigin {
        anchor: DiagnosticAnchor,
        origin: String,
    },

    /// A borrowed value remains live across a suspension point.
    #[diagnostic(code = "EV301", message = "borrow crosses suspension point")]
    BorrowAcrossSuspend {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },
}
