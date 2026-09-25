use destack_artifact_macros::Diagnostic;

use crate::DiagnosticAnchor;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Verify)]
pub enum VerifyError {
    // move checking
    /// A place is read before it is initialized.
    ///
    /// ```mir
    /// function test(): int32 {
    ///     local l0: int32
    ///
    /// entry:
    ///     v0: int32 = local.get l0
    ///     return v0
    /// }
    /// ```
    #[diagnostic(
        id = "use-of-uninitialized-place",
        message = "use of uninitialized value"
    )]
    UseOfUninitializedPlace {
        /// The invalid use.
        anchor: DiagnosticAnchor,
    },

    /// A place is initialized on only some incoming control-flow paths.
    ///
    /// ```mir
    /// function test(v0: boolean, v1: int32): int32 {
    ///     local l0: int32
    ///
    /// entry(v0: boolean, v1: int32):
    ///     branch v0 => initialize | skip
    ///
    /// initialize:
    ///     local.set l0, v1
    ///     jump done
    ///
    /// skip:
    ///     jump done
    ///
    /// done:
    ///     v2: int32 = local.get l0
    ///     return v2
    /// }
    /// ```
    #[diagnostic(
        id = "maybe-use-of-uninitialized-place",
        message = "value may be uninitialized"
    )]
    MaybeUseOfUninitializedPlace {
        /// The invalid use.
        anchor: DiagnosticAnchor,
    },

    /// A value is used after ownership was transferred.
    ///
    /// ```mir
    /// external function consume(ref<int32, unique, mutable>): void
    ///
    /// function test(v0: ref<int32, unique, mutable>): int32 {
    /// entry(v0: ref<int32, unique, mutable>):
    ///     call consume(v0): (ref<int32, unique, mutable>) => void
    ///     v1: int32 = load v0
    ///     return v1
    /// }
    /// ```
    #[diagnostic(id = "use-after-move", message = "use of moved value")]
    UseAfterMove {
        /// The use after the move.
        anchor: DiagnosticAnchor,
        /// The original move.
        moved_at: DiagnosticAnchor,
    },

    /// A value may have been moved on one control-flow path.
    ///
    /// ```mir
    /// external function consume(ref<int32, unique, mutable>): void
    ///
    /// function test(v0: ref<int32, unique, mutable>, v1: boolean): int32 {
    /// entry(v0: ref<int32, unique, mutable>, v1: boolean):
    ///     branch v1 => consume | preserve
    ///
    /// consume:
    ///     call consume(v0): (ref<int32, unique, mutable>) => void
    ///     jump done
    ///
    /// preserve:
    ///     jump done
    ///
    /// done:
    ///     v2: int32 = load v0
    ///     return v2
    /// }
    /// ```
    #[diagnostic(id = "maybe-use-after-move", message = "value may have been moved")]
    MaybeUseAfterMove {
        /// The use after a possible move.
        anchor: DiagnosticAnchor,
        /// The possible move.
        moved_at: DiagnosticAnchor,
    },

    /// A field is moved out of a type that implements Drop.
    ///
    /// ```mir
    /// type Row {
    ///     left: ref<int32, unique, mutable>;
    ///     right: ref<int32, unique, mutable>;
    /// }
    ///
    /// external function dropRow(ref<Row, borrowed, mutable>): void // Row drop hook
    ///
    /// function test(v0: Row): ref<int32, unique, mutable> {
    /// entry(v0: Row):
    ///     v1: ref<int32, unique, mutable> = field.get v0, 0
    ///     return v1
    /// }
    /// ```
    #[diagnostic(
        id = "move-out-of-drop",
        message = "cannot move out of a value that implements Drop"
    )]
    MoveOutOfDrop {
        /// The projected move.
        anchor: DiagnosticAnchor,
    },

    /// A safe load cannot move a value out through a reference.
    ///
    /// ```mir
    /// type Box {
    ///     value: ref<int32, unique, mutable>;
    /// }
    ///
    /// function test(v0: ref<Box, borrowed, mutable>): void {
    /// entry(v0: ref<Box, borrowed, mutable>):
    ///     v1: ref<ref<int32, unique, mutable>, borrowed, mutable> = field.address v0, 0
    ///     v2: ref<int32, unique, mutable> = load v1
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "move-out-of-reference",
        message = "cannot move out through a reference"
    )]
    MoveOutOfReference {
        /// The invalid load.
        anchor: DiagnosticAnchor,
    },

    /// A constructor returns before initializing one field.
    #[diagnostic(
        id = "field-left-uninitialized",
        message = "constructor returns before initializing field {field}"
    )]
    FieldLeftUninitialized {
        /// The returning path.
        anchor: DiagnosticAnchor,
        /// The uninitialized field position.
        field: String,
    },

    /// A constructor initializes one field twice.
    #[diagnostic(
        id = "field-initialized-twice",
        message = "constructor initializes field {field} twice"
    )]
    FieldInitializedTwice {
        /// The repeated store.
        anchor: DiagnosticAnchor,
        /// The repeated field position.
        field: String,
    },

    /// A constructor reads its receiver as the constructed object before every field initializes.
    #[diagnostic(
        id = "receiver-before-initialization",
        message = "'this' escapes before every field initializes"
    )]
    ReceiverBeforeInitialization {
        /// The escaping read.
        anchor: DiagnosticAnchor,
    },

    /// A constructor delegates to its base constructor twice.
    #[diagnostic(
        id = "super-called-twice",
        message = "constructor delegates to its base constructor twice"
    )]
    SuperCalledTwice {
        /// The repeated delegation.
        anchor: DiagnosticAnchor,
    },

    // borrow checking
    /// A new borrow conflicts with an active borrow.
    ///
    /// ```mir
    /// type Box {
    ///     value: int32;
    /// }
    ///
    /// function test(v0: ref<Box, borrowed, mutable>): int32 {
    /// entry(v0: ref<Box, borrowed, mutable>):
    ///     v1: ref<int32, borrowed, readonly> = field.address v0, 0
    ///     v2: ref<int32, borrowed, mutable> = field.address v0, 0
    ///     v3: int32 = load v1
    ///     return v3
    /// }
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

    /// A borrow requests stronger access than its source grants.
    ///
    /// ```mir
    /// type Box {
    ///     value: int32;
    /// }
    ///
    /// function test(v0: ref<Box, borrowed, readonly>): void {
    /// entry(v0: ref<Box, borrowed, readonly>):
    ///     v1: ref<int32, borrowed, mutable> = address (*v0).0
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "borrow-access-strengthening",
        message = "cannot strengthen borrowed access"
    )]
    BorrowAccessStrengthening {
        /// The invalid borrow.
        anchor: DiagnosticAnchor,
    },

    /// An alias can replace the case containing a borrowed inline payload.
    ///
    /// ```mir
    /// type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };
    ///
    /// function test(v0: ref<Either, borrowed, mutable>): void {
    /// entry(v0: ref<Either, borrowed, mutable>):
    ///     v1: ref<int32, borrowed, mutable> = address ((*v0) as 0)
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "borrow-of-aliasable-variant",
        message = "cannot borrow an inline variant payload through aliasable access"
    )]
    BorrowOfAliasableVariant {
        /// The payload borrow.
        anchor: DiagnosticAnchor,
    },

    /// Exclusive call arguments overlap.
    ///
    /// ```mir
    /// external function update(ref<int32, borrowed, mutable>, ref<int32, borrowed, mutable>): void
    ///
    /// function test(v0: ref<int32, borrowed, mutable>): void {
    /// entry(v0: ref<int32, borrowed, mutable>):
    ///     call update(v0, v0): (ref<int32, borrowed, mutable>, ref<int32, borrowed, mutable>) => void
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "mutable-argument-alias",
        message = "mutable call arguments may refer to the same owned storage"
    )]
    MutableArgumentAlias {
        /// The invalid call.
        anchor: DiagnosticAnchor,
    },

    /// A place is invalidated while an overlapping loan is live.
    ///
    /// ```mir
    /// function test(v0: int32, v1: int32): int32 {
    ///     local l0: int32
    ///
    /// entry(v0: int32, v1: int32):
    ///     local.set l0, v0
    ///     v2: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     local.set l0, v1
    ///     v3: int32 = load v2
    ///     return v3
    /// }
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

    /// A place cannot be read through another reference during a mutable borrow of it.
    ///
    /// ```mir
    /// function test(v0: int32): int32 {
    ///     local l0: int32
    ///
    /// entry(v0: int32):
    ///     local.set l0, v0
    ///     v1: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v2: int32 = local.get l0
    ///     return v2
    /// }
    /// ```
    #[diagnostic(
        id = "use-of-exclusively-borrowed-place",
        message = "cannot use exclusively borrowed place"
    )]
    UseOfExclusivelyBorrowedPlace {
        /// The conflicting read.
        anchor: DiagnosticAnchor,
        /// The mutable borrow.
        borrowed_at: DiagnosticAnchor,
    },

    /// A MIR store cannot write through a readonly reference.
    ///
    /// ```mir
    /// function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
    /// entry(v0: ref<int32, borrowed, readonly>, v1: int32):
    ///     store v0, v1
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "write-through-readonly-reference",
        message = "invalid MIR: cannot write through readonly reference"
    )]
    WriteThroughReadonlyReference {
        /// The readonly store.
        anchor: DiagnosticAnchor,
    },

    /// Exclusive borrowed access cannot be created from shared storage.
    ///
    /// ```mir
    /// type User {
    ///     id: int32;
    /// }
    ///
    /// function test(v0: ref<User, managed, mutable, shared>): int32 {
    /// entry(v0: ref<User, managed, mutable, shared>):
    ///     v1: ref<int32, borrowed, 'managed, mutable> = address (*v0).0
    ///     v2: int32 = load (*v1)
    ///     return v2
    /// }
    /// ```
    #[diagnostic(
        id = "mutable-borrow-from-shared-storage",
        message = "cannot borrow shared storage mutably"
    )]
    MutableBorrowFromSharedStorage {
        /// The mutable borrow.
        anchor: DiagnosticAnchor,
    },

    /// An escaping borrow is not covered by the required lifetime.
    ///
    /// ```mir
    /// function test(): ref<int32, borrowed, readonly> {
    ///     local l0: int32
    ///
    /// entry:
    ///     v0: ref<int32, borrowed, readonly, frame> = local.address l0
    ///     return v0
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

    /// A drop hook carries effects a drop may not perform.
    ///
    /// ```mir
    /// type Box {
    ///     value: int32;
    /// }
    ///
    /// external function effectful(): void
    ///
    /// function dropBox(v0: ref<Box, borrowed, mutable>): void {
    /// entry(v0: ref<Box, borrowed, mutable>):
    ///     call effectful(): () => void
    ///     return
    /// }
    /// ```
    #[diagnostic(
        id = "drop-effect",
        message = "this drop may park",
        help = "move the parking work to an explicit dispose"
    )]
    DropEffect {
        /// The drop function.
        anchor: DiagnosticAnchor,
    },
}
