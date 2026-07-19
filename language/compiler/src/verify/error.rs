use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Verify)]
pub enum VerifyError {
    /// Value used after ownership was transferred.
    ///
    /// ```mir
    /// v1: ref<int32, unique, mutable> = field.get v0, 0
    /// v2: ref<int32, unique, mutable> = field.get v0, 0 // moved by v1
    /// ```
    #[diagnostic(id = "use-after-move", message = "use of moved value")]
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
    #[diagnostic(id = "maybe-use-after-move", message = "value may have been moved")]
    MaybeUseAfterMove {
        /// The use after a possible move.
        anchor: DiagnosticAnchor,
        /// The possible move.
        moved_at: DiagnosticAnchor,
    },

    /// An aggregate remains partially moved before another operation.
    ///
    /// ```mir
    /// v1: Row = aggregate (v0, v2)
    /// v3: ref<int32, unique, mutable> = field.get v1, 0
    /// call consume(v3) // v1 still owns its other field
    /// ```
    #[diagnostic(id = "partial-move", message = "aggregate is only partially moved")]
    PartialMove {
        /// The operation reached before decomposition completed.
        anchor: DiagnosticAnchor,
        /// The projection that began decomposition.
        moved_at: DiagnosticAnchor,
    },

    /// A Drop method cannot suspend execution.
    #[diagnostic(id = "drop-may-suspend", message = "Drop method may suspend")]
    DropMaySuspend {
        /// The Drop method.
        anchor: DiagnosticAnchor,
    },

    /// A Drop method cannot panic.
    #[diagnostic(id = "drop-may-panic", message = "Drop method may panic")]
    DropMayPanic {
        /// The Drop method.
        anchor: DiagnosticAnchor,
    },

    /// A Drop method cannot allocate storage.
    #[diagnostic(id = "drop-may-allocate", message = "Drop method may allocate")]
    DropMayAllocate {
        /// The Drop method.
        anchor: DiagnosticAnchor,
    },

    /// A Drop method cannot observe entropy or host state.
    #[diagnostic(
        id = "drop-may-observe-entropy",
        message = "Drop method may observe entropy"
    )]
    DropMayObserveEntropy {
        /// The Drop method.
        anchor: DiagnosticAnchor,
    },

    /// Cannot move a field out of a type that implements Drop.
    #[diagnostic(
        id = "move-out-of-drop",
        message = "cannot move out of a value that implements Drop"
    )]
    MoveOutOfDrop {
        /// The projected move.
        anchor: DiagnosticAnchor,
    },

    /// A Drop method must exclusively borrow its receiver and return void.
    #[diagnostic(
        id = "invalid-drop-signature",
        message = "Drop method must take one exclusive borrowed receiver and return void"
    )]
    InvalidDropSignature {
        /// The invalid Drop method.
        anchor: DiagnosticAnchor,
    },

    /// A new borrow conflicts with an active borrow.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    /// v2: ref<int32, borrowed, readonly> = field.address v0, 0 // overlaps v1
    /// ```
    #[diagnostic(
        id = "borrow-conflict",
        message = "borrow conflicts with active borrow"
    )]
    BorrowConflict {
        /// The new borrow.
        anchor: DiagnosticAnchor,
        /// The active borrow.
        active_borrow: DiagnosticAnchor,
    },

    /// Cannot invalidate a place while an overlapping loan is live.
    ///
    /// ```mir
    /// v1: ref<int32, borrowed, mutable> = field.address v0, 0
    /// call consume(v0) // moves the borrowed root
    /// ```
    #[diagnostic(
        id = "invalidation-of-borrowed-place",
        message = "cannot invalidate borrowed place"
    )]
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
        id = "write-through-readonly-reference",
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
    /// // v0: ref<User, managed, mutable, space(shared)>
    /// ```
    #[diagnostic(
        id = "exclusive-borrow-from-shared-managed",
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
    /// yield v2 => b1(v1) // v0 is managed
    /// ```
    #[diagnostic(
        id = "managed-borrow-across-suspension",
        message = "managed borrow cannot cross suspension"
    )]
    ManagedBorrowAcrossSuspension {
        /// The suspension point.
        anchor: DiagnosticAnchor,
        /// The managed-rooted borrow.
        borrowed_at: DiagnosticAnchor,
    },

    /// An escaping borrow is not covered by the required lifetime.
    ///
    /// ```mir
    /// function test(v0: ref<User, managed, mutable>): ref<int32, borrowed, lifetime(static), mutable> {
    ///     v1: ref<int32, borrowed, mutable> = field.address v0, 0
    ///     return v1 // managed borrow is not static
    /// }
    /// ```
    #[diagnostic(
        id = "borrow-outlives-origin",
        message = "borrow does not live long enough"
    )]
    BorrowOutlivesOrigin {
        /// The escaping borrow.
        anchor: DiagnosticAnchor,
    },

    /// A function body requires a borrow obligation not declared by its signature.
    ///
    /// ```mir
    /// function test(v0: ref<int32, borrowed, readonly>): void {
    ///     yield v0 => b1(v0) // v0 needs @suspensionSafe
    /// }
    /// ```
    #[diagnostic(
        id = "undeclared-borrow-obligation",
        message = "invalid MIR: function signature is missing borrow obligation"
    )]
    UndeclaredBorrowObligation {
        /// The function with an incomplete signature.
        anchor: DiagnosticAnchor,
    },
}
