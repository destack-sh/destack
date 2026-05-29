use crate::DiagnosticAnchor;
use crate::tests::TestProgram;
use crate::verify::{BorrowObligationRecord, VerifyError};
use destack_mir as mir;

impl TestProgram {
    /// Return the only ownership error.
    fn one_ownership_error<'a>(&self, errors: &'a [VerifyError]) -> &'a VerifyError {
        assert_eq!(errors.len(), 1, "{errors:#?}");

        &errors[0]
    }

    /// Return the byte start for one diagnostic anchor.
    fn anchor_start(&self, anchor: &DiagnosticAnchor) -> u32 {
        let DiagnosticAnchor::Span(span) = anchor else {
            panic!("expected span anchor, got {anchor:#?}");
        };

        span.start
    }

    /// Assert that `first` points before `second`.
    fn assert_anchor_before(&self, first: &DiagnosticAnchor, second: &DiagnosticAnchor) {
        assert!(
            self.anchor_start(first) < self.anchor_start(second),
            "expected {first:#?} before {second:#?}",
        );
    }

    /// Assert no ownership errors.
    fn assert_no_ownership_errors(&mut self) {
        let errors = self.run_ownership();

        assert!(errors.is_empty(), "{errors:#?}");
    }

    /// Assert one definite use after move.
    fn assert_use_after_move(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::UseAfterMove { anchor, moved_at } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        self.assert_anchor_before(moved_at, anchor);
    }

    /// Assert one maybe use after move.
    fn assert_maybe_use_after_move(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::MaybeUseAfterMove { anchor, moved_at } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        self.assert_anchor_before(moved_at, anchor);
    }

    /// Assert one conflicting borrow.
    fn assert_borrow_conflict(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::BorrowConflict {
            anchor,
            active_borrow,
        } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        self.assert_anchor_before(active_borrow, anchor);
    }

    /// Assert one borrowed-place change.
    fn assert_change_of_borrowed_place(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::ChangeOfBorrowedPlace {
            anchor,
            borrowed_at,
        } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        self.assert_anchor_before(borrowed_at, anchor);
    }

    /// Assert one readonly write.
    fn assert_readonly_write(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::ReadonlyWrite { anchor } = self.one_ownership_error(&errors) else {
            panic!("{errors:#?}");
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one exclusive borrow crossing suspension.
    fn assert_exclusive_borrow_across_suspension(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::ExclusiveBorrowAcrossSuspension {
            anchor,
            borrowed_at,
        } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        self.assert_anchor_before(borrowed_at, anchor);
    }

    /// Assert one rejected exclusive shared managed borrow.
    fn assert_exclusive_borrow_from_shared_managed(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::ExclusiveBorrowFromSharedManaged { anchor } =
            self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one borrow crossing suspension.
    fn assert_borrow_across_suspension(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::BorrowAcrossSuspension {
            anchor,
            borrowed_at,
        } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
        assert!(
            matches!(borrowed_at, DiagnosticAnchor::Span(_)),
            "{borrowed_at:#?}"
        );
    }

    /// Assert one escaping borrow.
    fn assert_borrow_outlives_origin(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::BorrowOutlivesOrigin { anchor } = self.one_ownership_error(&errors) else {
            panic!("{errors:#?}");
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one rejected partial move of a custom drop type.
    fn assert_partial_move_of_custom_drop(&mut self) {
        let errors = self.run_ownership();
        let VerifyError::PartialMoveOfCustomDrop { anchor } = self.one_ownership_error(&errors)
        else {
            panic!("{errors:#?}");
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one suspension-stable borrow obligation.
    fn assert_suspension_stable_obligation(&mut self, parameter: u32) {
        let (errors, obligations) = self.run_ownership_obligations();

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(obligations.len(), 1, "{obligations:#?}");

        let BorrowObligationRecord { obligation, anchor } = &obligations[0];
        let mir::BorrowObligation::SuspensionStable { lifetime } = obligation;
        assert!(lifetime.includes_parameter(parameter), "{lifetime:#?}");
        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert no ownership errors and return borrow obligations.
    fn assert_ownership_obligations(&mut self) -> Vec<BorrowObligationRecord> {
        let (errors, obligations) = self.run_ownership_obligations();

        assert!(errors.is_empty(), "{errors:#?}");

        obligations
    }
}

#[test]
fn test_reject_exclusive_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
    );

    program.assert_borrow_conflict();
}

#[test]
fn test_allow_aliasable_mutable_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_exclusive_disjoint_fields() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    int32;
    int32;
}

function test(v0: ref<Pair, borrowed>): void {
b0(v0: ref<Pair, borrowed>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 1
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_exclusive_dynamic_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4], v1: usize, v2: usize): void {
b0(v0: [int32; 4], v1: usize, v2: usize):
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}"#,
    );

    program.assert_borrow_conflict();
}

#[test]
fn test_allow_exclusive_constant_element_disjoint() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4]): void {
b0(v0: [int32; 4]):
    v1: uint64 = 0uint64
    v2: uint64 = 1uint64
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_exclusive_borrowed_slice_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed>, v1: usize, v2: usize): void {
b0(v0: slice<int32, borrowed>, v1: usize, v2: usize):
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}"#,
    );

    program.assert_borrow_conflict();
}

#[test]
fn test_allow_exclusive_disjoint_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed>): void {
b0(v0: slice<int32, borrowed>):
    v1: uint64 = 0uint64
    v2: uint64 = 2uint64
    v3: slice<int32, borrowed, exclusive> = slice v0, v1, v2
    v4: slice<int32, borrowed, exclusive> = slice v0, v2, v2
    v5: ref<int32, borrowed, exclusive> = element.address v3, v1
    v6: ref<int32, borrowed, exclusive> = element.address v4, v1
    v7: int32 = load v5
    v8: int32 = load v6
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_exclusive_overlapping_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed>): void {
b0(v0: slice<int32, borrowed>):
    v1: uint64 = 0uint64
    v2: uint64 = 2uint64
    v3: uint64 = 1uint64
    v4: slice<int32, borrowed, exclusive> = slice v0, v1, v2
    v5: slice<int32, borrowed, exclusive> = slice v0, v3, v2
    v6: ref<int32, borrowed, exclusive> = element.address v4, v1
    v7: ref<int32, borrowed, exclusive> = element.address v5, v1
    v8: int32 = load v6
    v9: int32 = load v7
    return
}"#,
    );

    program.assert_borrow_conflict();
}

#[test]
fn test_reject_exclusive_after_readonly_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
    );

    program.assert_borrow_conflict();
}

#[test]
fn test_allow_exclusive_after_last_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, exclusive> = field.address v0, 0
    v4: int32 = load v3
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_move_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function consume(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    return
}

function test(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    v1: ref<int32, borrowed> = field.address v0, 0
    call consume(v0): (ref<Box, unique>) -> void
    v2: int32 = load v1
    return
}"#,
    );

    program.assert_change_of_borrowed_place();
}

#[test]
fn test_reject_projected_move_use_after_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    ref<int32, unique>;
    ref<int32, unique>;
}

function test(v0: ref<int32, unique>, v1: ref<int32, unique>): void {
b0(v0: ref<int32, unique>, v1: ref<int32, unique>):
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique> = field.get v2, 0
    v4: ref<int32, unique> = field.get v2, 0
    return
}"#,
    );

    program.assert_use_after_move();
}

#[test]
fn test_allow_disjoint_field_use_after_projected_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    ref<int32, unique>;
    ref<int32, unique>;
}

function test(v0: ref<int32, unique>, v1: ref<int32, unique>): void {
b0(v0: ref<int32, unique>, v1: ref<int32, unique>):
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique> = field.get v2, 0
    v4: ref<int32, unique> = field.get v2, 1
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_union_use_after_payload_move() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, unique>> { 0uint8 = ref<int32, unique>; 1uint8 = int32; }

function test(v0: Value): void {
b0(v0: Value):
    v1: ref<int32, unique> = field.get v0, 0
    v2: int32 = field.get v0, 1
    return
}"#,
    );

    program.assert_use_after_move();
}

#[test]
fn test_reject_partial_move_of_custom_drop() {
    let mut program = TestProgram::mir(
        r#"
type Row {
    ref<int32, unique>;
    ref<int32, unique>;
}

function test(v0: ref<int32, unique>, v1: ref<int32, unique>): void {
b0(v0: ref<int32, unique>, v1: ref<int32, unique>):
    v2: Row = struct Row (v0, v1)
    v3: ref<int32, unique> = field.get v2, 0
    return
}"#,
    );
    program.mark_dynamic_drop("Row");

    program.assert_partial_move_of_custom_drop();
}

#[test]
fn test_reject_move_while_borrowed_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function consume(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    return
}

function test(v0: ref<Box, unique>, v1: boolean): void {
b0(v0: ref<Box, unique>, v1: boolean):
    v2: ref<int32, borrowed> = field.address v0, 0
    branch v1, b1(v2), b2
b1(v3: ref<int32, borrowed>):
    call consume(v0): (ref<Box, unique>) -> void
    v4: int32 = load v3
    return
b2:
    return
}"#,
    );

    program.assert_change_of_borrowed_place();
}

#[test]
fn test_reject_local_set_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32, v1: int32): void {
    local local0: int32, owned
b0(v0: int32, v1: int32):
    local.set local0, v0
    v2: ref<int32, borrowed, space(frame)> = local.address local0
    local.set local0, v1
    v3: int32 = load v2
    return
}"#,
    );

    program.assert_change_of_borrowed_place();
}

#[test]
fn test_reject_store_through_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
b0(v0: ref<int32, borrowed, readonly>, v1: int32):
    store v0, v1
    return
}"#,
    );

    program.assert_readonly_write();
}

#[test]
fn test_reject_store_while_exclusive_borrow_is_live() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>, v1: int32): void {
b0(v0: ref<Box, borrowed>, v1: int32):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: Box = struct Box (v1)
    store v0, v3
    v4: int32 = load v2
    return
}"#,
    );

    program.assert_change_of_borrowed_place();
}

#[test]
fn test_allow_store_through_own_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(v0: ref<Box, borrowed>, v1: int32): void {
b0(v0: ref<Box, borrowed>, v1: int32):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_store_through_live_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>, v1: int32): void {
b0(v0: ref<int32, borrowed>, v1: int32):
    store v0, v1
    v2: int32 = load v0
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_use_after_call_move() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    call consume(v0): (ref<int32, unique>) -> void
    v1: int32 = load v0
    return
}"#,
    );

    program.assert_use_after_move();
}

#[test]
fn test_reject_use_after_struct_move() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    ref<int32, unique>;
}

function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: Box = struct Box (v0)
    v2: int32 = load v0
    return
}"#,
    );

    program.assert_use_after_move();
}

#[test]
fn test_reject_use_after_new_complete() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(): void {
b0:
    v0: uninit<ref<Box, managed>> = new.uninit Box
    v1: ref<Box, managed> = new.complete v0
    v2: ref<Box, managed> = new.complete v0
    return
}"#,
    );

    program.assert_use_after_move();
}

#[test]
fn test_ignore_raw_free_for_moves() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    return
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_frame_return() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    int32;
}

function test(): ref<int32, borrowed> {
b0:
    v0: ref<Box, raw, space(frame)> = frame.alloc.zeroed Box
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_allow_parameter_return_when_declared() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_infer_parameter_return_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_infer_managed_return_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): ref<int32, borrowed, readonly> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_infer_managed_slice_return_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, managed>): ref<int32, borrowed, readonly> {
b0(v0: slice<int32, managed>):
    v1: int64 = 0int64
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>, v1: boolean): ref<int32, borrowed, readonly> {
b0(v0: ref<User, managed>, v1: boolean):
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    branch v1, b1(v2), b2(v2)
b1(v3: ref<int32, borrowed, readonly>):
    return v3
b2(v4: ref<int32, borrowed, readonly>):
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, managed>, v1: boolean): ref<int32, borrowed, readonly> {
b0(v0: slice<int32, managed>, v1: boolean):
    v2: int64 = 0int64
    v3: ref<int32, borrowed, readonly> = element.address v0, v2
    branch v1, b1(v3), b2(v3)
b1(v4: ref<int32, borrowed, readonly>):
    return v4
b2(v5: ref<int32, borrowed, readonly>):
    return v5
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_managed_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): int32 {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = 0int32
    yield v2, b1(v0, v1)
b1(v3: ref<User, managed>, v4: ref<int32, borrowed, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_reject_managed_borrow_yield_value() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): int32 {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    yield v1, b1(v0)
b1(v2: ref<User, managed>):
    v3: int32 = 0int32
    return v3
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_allow_shared_managed_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed, space(shared)>): int32 {
b0(v0: ref<User, managed, space(shared)>):
    v1: ref<int32, borrowed, readonly, space(shared)> = field.address v0, 0
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_shared_managed_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed, space(shared)>): int32 {
b0(v0: ref<User, managed, space(shared)>):
    v1: ref<int32, borrowed, exclusive, space(shared)> = field.address v0, 0
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_exclusive_borrow_from_shared_managed();
}

#[test]
fn test_reject_shared_managed_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed, space(shared)>): int32 {
b0(v0: ref<User, managed, space(shared)>):
    v1: ref<int32, borrowed, readonly, space(shared)> = field.address v0, 0
    v2: int32 = 0int32
    yield v2, b1(v0, v1)
b1(v3: ref<User, managed, space(shared)>, v4: ref<int32, borrowed, readonly, space(shared)>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_allow_shared_unique_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, unique, space(shared)>): int32 {
b0(v0: ref<User, unique, space(shared)>):
    v1: ref<int32, borrowed, readonly, space(shared)> = field.address v0, 0
    v2: int32 = 0int32
    yield v2, b1(v0, v1)
b1(v3: ref<User, unique, space(shared)>, v4: ref<int32, borrowed, readonly, space(shared)>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_unique_slice_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, unique>): ref<int32, borrowed> {
b0(v0: slice<int32, unique>):
    v1: int64 = 0int64
    v2: ref<int32, borrowed> = element.address v0, v1
    return v2
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_unique_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, unique>): ref<int32, borrowed> {
b0(v0: ref<User, unique>):
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_wrong_managed_parameter_lifetime_return() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>, v1: ref<User, managed>): ref<int32, borrowed, readonly, lifetime(0)> {
b0(v0: ref<User, managed>, v1: ref<User, managed>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    return v2
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_managed_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): ref<int32, borrowed, readonly, lifetime(static)> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_aggregate_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    ref<int32, borrowed>;
}

function test(v0: Box): ref<int32, borrowed, lifetime(static)> {
b0(v0: Box):
    v1: ref<int32, borrowed> = field.get v0, 0
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_union_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, borrowed>> { 0uint8 = ref<int32, borrowed>; 1uint8 = int32; }

function test(v0: Value): ref<int32, borrowed, lifetime(static)> {
b0(v0: Value):
    v1: ref<int32, borrowed> = field.get v0, 1
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_allow_frame_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local local0: int32, owned
entry0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    yield v0, block1(v1)
block1(v2: int32, v3: ref<int32, borrowed, space(frame)>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_require_mutable_parameter_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>): int32 {
entry0(v0: ref<int32, borrowed>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_suspension_stable_obligation(0);
}

#[test]
fn test_require_readonly_parameter_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, readonly>): int32 {
entry0(v0: ref<int32, borrowed, readonly>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed, readonly>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_suspension_stable_obligation(0);
}

#[test]
fn test_allow_static_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, readonly, lifetime(static)>): int32 {
entry0(v0: ref<int32, borrowed, readonly, lifetime(static)>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed, readonly, lifetime(static)>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_call_obligation_from_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function callee(v0: ref<int32, borrowed, readonly>): int32 {
b0(v0: ref<int32, borrowed, readonly>):
    v1: int32 = 0int32
    yield v1, b1(v0)
b1(v2: ref<int32, borrowed, readonly>):
    v3: int32 = load v2
    return v3
}

function caller(v0: ref<User, managed>): int32 {
b2(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = call callee(v1): (ref<int32, borrowed, readonly>) -> int32
    return v2
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_reject_call_result_borrow_from_managed_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function callee(v0: ref<User, managed>): ref<int32, borrowed, readonly> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}

function caller(v0: ref<User, managed>): int32 {
b1(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = call callee(v0): (ref<User, managed>) -> ref<int32, borrowed, readonly>
    v2: int32 = 0int32
    yield v2, b2(v0, v1)
b2(v3: ref<User, managed>, v4: ref<int32, borrowed, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_reject_tail_call_result_borrow_from_managed_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function callee(v0: ref<User, managed>): ref<int32, borrowed, readonly> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}

function caller(v0: ref<User, managed>): ref<int32, borrowed, readonly> {
b1(v0: ref<User, managed>):
    tailCall callee(v0): (ref<User, managed>) -> ref<int32, borrowed, readonly>
}

function outer(v0: ref<User, managed>): int32 {
b2(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = call caller(v0): (ref<User, managed>) -> ref<int32, borrowed, readonly>
    v2: int32 = 0int32
    yield v2, b3(v0, v1)
b3(v3: ref<User, managed>, v4: ref<int32, borrowed, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_propagate_call_obligation_from_borrowed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function callee(v0: ref<int32, borrowed, readonly>): int32 {
b0(v0: ref<int32, borrowed, readonly>):
    v1: int32 = 0int32
    yield v1, b1(v0)
b1(v2: ref<int32, borrowed, readonly>):
    v3: int32 = load v2
    return v3
}

function caller(v0: ref<int32, borrowed, readonly>): int32 {
b2(v0: ref<int32, borrowed, readonly>):
    v1: int32 = call callee(v0): (ref<int32, borrowed, readonly>) -> int32
    return v1
}"#,
    );

    let obligations = program.assert_ownership_obligations();
    assert_eq!(obligations.len(), 2, "{obligations:#?}");

    // require the same source proof from callee and caller
    for BorrowObligationRecord { obligation, anchor } in &obligations {
        let mir::BorrowObligation::SuspensionStable { lifetime } = obligation;
        assert!(lifetime.includes_parameter(0), "{lifetime:#?}");
        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }
}

#[test]
fn test_reject_indirect_call_obligation_from_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function caller(v0: (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0), v1: ref<User, managed>): int32 {
b0(v0: (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0), v1: ref<User, managed>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    v3: int32 = call.indirect v0(v2): (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0)
    return v3
}"#,
    );

    program.assert_borrow_across_suspension();
}

#[test]
fn test_propagate_indirect_call_obligation_from_borrowed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function caller(v0: (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0), v1: ref<int32, borrowed, readonly>): int32 {
b0(v0: (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0), v1: ref<int32, borrowed, readonly>):
    v2: int32 = call.indirect v0(v1): (ref<int32, borrowed, readonly>) -> int32 @suspensionSafe(0)
    return v2
}"#,
    );

    program.assert_suspension_stable_obligation(1);
}

#[test]
fn test_require_call_result_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function callee(v0: ref<int32, borrowed, readonly>): ref<int32, borrowed, readonly> {
b0(v0: ref<int32, borrowed, readonly>):
    return v0
}

function caller(v0: ref<int32, borrowed, readonly>): int32 {
b1(v0: ref<int32, borrowed, readonly>):
    v1: ref<int32, borrowed, readonly> = call callee(v0): (ref<int32, borrowed, readonly>) -> ref<int32, borrowed, readonly>
    v2: int32 = 0int32
    yield v2, b2(v1)
b2(v3: ref<int32, borrowed, readonly>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_suspension_stable_obligation(0);
}

#[test]
fn test_reject_exclusive_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local local0: int32, owned
entry0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed, exclusive, space(frame)> = local.address local0
    yield v0, block1(v1)
block1(v2: int32, v3: ref<int32, borrowed, exclusive, space(frame)>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_exclusive_borrow_across_suspension();
}

#[test]
fn test_reject_exclusive_parameter_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, exclusive>): int32 {
entry0(v0: ref<int32, borrowed, exclusive>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed, exclusive>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_exclusive_borrow_across_suspension();
}

#[test]
fn test_allow_borrow_before_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local local0: int32, owned
entry0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: int32 = load v1
    yield v2, block1(v2)
block1(v3: int32, v4: int32):
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_static_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
readonly global value: int32 = 1int32
function test(): ref<int32, borrowed, lifetime(static)> {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: ref<int32, borrowed, readonly, lifetime(static)> = cast.bit v0 -> ref<int32, borrowed, readonly, lifetime(static)>
    return v1
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_wrong_parameter_lifetime_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v1
}"#,
    );

    program.assert_borrow_outlives_origin();
}

#[test]
fn test_reject_maybe_moved_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>, v1: boolean): void {
b0(v0: ref<int32, unique>, v1: boolean):
    branch v1, b1, b2
b1:
    call consume(v0): (ref<int32, unique>) -> void
    jump b3
b2:
    jump b3
b3:
    v2: int32 = load v0
    return
}"#,
    );

    program.assert_maybe_use_after_move();
}

#[test]
fn test_carry_lifetime_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>, v1: boolean): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: boolean):
    branch v1, b1(v0), b2(v0)
b1(v2: ref<int32, borrowed>):
    return v2
b2(v3: ref<int32, borrowed>):
    return v3
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_merge_lifetimes_at_join() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>, v2: boolean): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>, v2: boolean):
    branch v2, b1, b2
b1:
    jump b3(v0)
b2:
    jump b3(v1)
b3(v3: ref<int32, borrowed>):
    return v3
}"#,
    );

    program.assert_borrow_outlives_origin();
}
