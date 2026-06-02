use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Verify)]
pub enum VerifyError {
    /// Value used after ownership was transferred.
    ///
    /// ```mir
    /// v1: ref<int32, unique> = field.get v0, 0
    /// v2: ref<int32, unique> = field.get v0, 0 // moved by v1
    /// ```
    #[diagnostic(code = "EV100", message = "use of moved value")]
    UseAfterMove {
        /// The use after the move.
        anchor: DiagnosticAnchor,
        /// The original move.
        moved_at: DiagnosticAnchor,
    },

    /// Value may have been moved on one control-flow path.
    ///
    /// ```mir
    /// branch v1, b1, b2
    /// b1:
    ///     call consume(v0)
    ///     jump b3
    /// b2:
    ///     jump b3
    /// b3:
    ///     load v0 // moved when reached through b1
    /// ```
    #[diagnostic(code = "EV101", message = "value may have been moved")]
    MaybeUseAfterMove {
        /// The use after a possible move.
        anchor: DiagnosticAnchor,
        /// The possible move.
        moved_at: DiagnosticAnchor,
    },

    /// Cannot partially move a type with custom drop glue.
    ///
    /// ```mir
    /// v1: Row = struct Row (v0, v2)
    /// v3: ref<int32, unique> = field.get v1, 0 // Row has custom drop
    /// ```
    #[diagnostic(
        code = "EV102",
        message = "cannot partially move value with custom drop"
    )]
    PartialMoveOfCustomDrop {
        /// The partial move.
        anchor: DiagnosticAnchor,
    },

    /// A new borrow conflicts with an active borrow.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    /// v2: ref<int32, borrowed, readonly> = field.address v0, 0 // overlaps v1
    /// ```
    #[diagnostic(code = "EV200", message = "borrow conflicts with active borrow")]
    BorrowConflict {
        /// The new borrow.
        anchor: DiagnosticAnchor,
        /// The active borrow.
        active_borrow: DiagnosticAnchor,
    },

    /// Cannot invalidate a place while an overlapping loan is live.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed> = field.address v0, 0
    /// call consume(v0) // moves the borrowed root
    /// ```
    #[diagnostic(code = "EV201", message = "cannot invalidate borrowed place")]
    InvalidationOfBorrowedPlace {
        /// The invalidating operation.
        anchor: DiagnosticAnchor,
        /// The active borrow.
        borrowed_at: DiagnosticAnchor,
    },

    /// A MIR store cannot write through a readonly reference.
    ///
    /// ```mir
    /// store v0, v1 // v0: ref<int32, borrowed, readonly>
    /// ```
    #[diagnostic(
        code = "EV202",
        message = "invalid MIR: cannot write through readonly reference"
    )]
    WriteThroughReadonlyReference {
        /// The readonly store.
        anchor: DiagnosticAnchor,
    },

    /// Exclusive borrowed access cannot be created from shared managed storage.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed, exclusive, space(shared)> = field.address v0, 0
    /// // v0: ref<User, managed, space(shared)>
    /// ```
    #[diagnostic(
        code = "EV204",
        message = "cannot borrow shared managed storage exclusively"
    )]
    ExclusiveBorrowFromSharedManaged {
        /// The exclusive borrow.
        anchor: DiagnosticAnchor,
    },

    /// Managed-rooted borrowed access cannot cross a suspension point.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed, readonly> = field.address v0, 0
    /// yield v2, b1(v1) // v0 is managed
    /// ```
    #[diagnostic(code = "EV205", message = "managed borrow cannot cross suspension")]
    ManagedBorrowAcrossSuspension {
        /// The suspension point.
        anchor: DiagnosticAnchor,
        /// The managed-rooted borrow.
        borrowed_at: DiagnosticAnchor,
    },

    /// An escaping borrow is not covered by the required lifetime.
    ///
    /// ```mir
    /// function test(v0: ref<User, managed>): ref<int32, borrowed, lifetime(static)> {
    ///     v1: ref<int32, borrowed> = field.address v0, 0
    ///     return v1 // managed borrow is not static
    /// }
    /// ```
    #[diagnostic(code = "EV300", message = "borrow does not live long enough")]
    BorrowOutlivesOrigin {
        /// The escaping borrow.
        anchor: DiagnosticAnchor,
    },
}
