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

    /// Cannot partially move a type with custom drop glue.
    #[diagnostic(
        code = "EV102",
        message = "cannot partially move value with custom drop"
    )]
    PartialMoveOfCustomDrop { anchor: DiagnosticAnchor },

    /// A new borrow conflicts with an active borrow.
    #[diagnostic(code = "EV200", message = "borrow conflicts with active borrow")]
    BorrowConflict {
        anchor: DiagnosticAnchor,
        active_borrow: DiagnosticAnchor,
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

    /// Exclusive borrowed access cannot cross a suspension point.
    #[diagnostic(code = "EV203", message = "exclusive borrow cannot cross suspension")]
    ExclusiveBorrowAcrossSuspension {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    /// Exclusive borrowed access cannot be created from shared managed storage.
    #[diagnostic(
        code = "EV204",
        message = "cannot borrow shared managed storage exclusively"
    )]
    ExclusiveBorrowFromSharedManaged { anchor: DiagnosticAnchor },

    /// Borrowed access must have an owned or static source to cross suspension.
    #[diagnostic(code = "EV205", message = "borrow cannot cross suspension")]
    BorrowAcrossSuspension {
        anchor: DiagnosticAnchor,
        borrowed_at: DiagnosticAnchor,
    },

    /// An escaping borrow is not covered by the required lifetime.
    #[diagnostic(code = "EV300", message = "borrow does not live long enough")]
    BorrowOutlivesOrigin { anchor: DiagnosticAnchor },
}
