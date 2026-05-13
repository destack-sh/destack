use crate::DiagnosticAnchor;
use crate::verify::VerifyError;
use crate::verify::tests::VerifyProgram;

/// Return the only verify error.
fn one_error(errors: &[VerifyError]) -> &VerifyError {
    assert_eq!(errors.len(), 1, "{errors:#?}");

    &errors[0]
}

/// Return the byte start for one diagnostic anchor.
fn anchor_start(anchor: &DiagnosticAnchor) -> u32 {
    let DiagnosticAnchor::Span(span) = anchor else {
        panic!("expected span anchor, got {anchor:#?}");
    };

    span.start
}

/// Assert that `first` points before `second`.
fn assert_anchor_before(first: &DiagnosticAnchor, second: &DiagnosticAnchor) {
    assert!(
        anchor_start(first) < anchor_start(second),
        "expected {first:#?} before {second:#?}",
    );
}

/// Assert one definite use after move.
fn assert_use_after_move(errors: &[VerifyError]) {
    let VerifyError::UseAfterMove { anchor, moved_at } = one_error(errors) else {
        panic!("{errors:#?}");
    };

    assert_anchor_before(moved_at, anchor);
}

/// Assert one maybe use after move.
fn assert_maybe_use_after_move(errors: &[VerifyError]) {
    let VerifyError::MaybeUseAfterMove { anchor, moved_at } = one_error(errors) else {
        panic!("{errors:#?}");
    };

    assert_anchor_before(moved_at, anchor);
}

/// Assert one conflicting loan.
fn assert_conflicting_loan(errors: &[VerifyError]) {
    let VerifyError::ConflictingLoan {
        anchor,
        existing_loan,
    } = one_error(errors)
    else {
        panic!("{errors:#?}");
    };

    assert_anchor_before(existing_loan, anchor);
}

/// Assert one borrowed-place change.
fn assert_change_of_borrowed_place(errors: &[VerifyError]) {
    let VerifyError::ChangeOfBorrowedPlace {
        anchor,
        borrowed_at,
    } = one_error(errors)
    else {
        panic!("{errors:#?}");
    };

    assert_anchor_before(borrowed_at, anchor);
}

/// Assert one readonly write.
fn assert_readonly_write(errors: &[VerifyError]) {
    let VerifyError::ReadonlyWrite { anchor } = one_error(errors) else {
        panic!("{errors:#?}");
    };

    assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
}

/// Assert one exclusive loan crossing suspension.
fn assert_exclusive_loan_across_suspension(errors: &[VerifyError]) {
    let VerifyError::ExclusiveLoanAcrossSuspension {
        anchor,
        borrowed_at,
    } = one_error(errors)
    else {
        panic!("{errors:#?}");
    };

    assert_anchor_before(borrowed_at, anchor);
}

/// Assert one escaping borrow.
fn assert_borrow_outlives_origin(errors: &[VerifyError]) {
    let VerifyError::BorrowOutlivesOrigin { anchor } = one_error(errors) else {
        panic!("{errors:#?}");
    };

    assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
}

/// Assert one rejected partial move of a drop type.
fn assert_partial_move_of_drop_type(errors: &[VerifyError]) {
    let VerifyError::PartialMoveOfDropType { anchor } = one_error(errors) else {
        panic!("{errors:#?}");
    };

    assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
}

/// Assert no verify errors.
fn assert_no_errors(errors: &[VerifyError]) {
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn test_reject_exclusive_overlap() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_conflicting_loan(&errors);
}

#[test]
fn test_allow_aliasable_mutable_overlap() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_exclusive_disjoint_fields() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_exclusive_dynamic_element_overlap() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: int32[4], v1: usize, v2: usize): void {
b0(v0: int32[4], v1: usize, v2: usize):
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_conflicting_loan(&errors);
}

#[test]
fn test_allow_exclusive_constant_element_disjoint() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint64 = 0uint64
    v2: uint64 = 1uint64
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_exclusive_borrowed_slice_element_overlap() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_conflicting_loan(&errors);
}

#[test]
fn test_allow_exclusive_disjoint_slice_ranges() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_exclusive_overlapping_slice_ranges() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_conflicting_loan(&errors);
}

#[test]
fn test_reject_exclusive_after_readonly_overlap() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_conflicting_loan(&errors);
}

#[test]
fn test_allow_exclusive_after_last_borrow_use() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_move_while_borrowed() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_change_of_borrowed_place(&errors);
}

#[test]
fn test_reject_projected_move_use_after_move() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_use_after_move(&errors);
}

#[test]
fn test_allow_disjoint_field_use_after_projected_move() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_union_use_after_payload_move() {
    let mut program = VerifyProgram::new(
        r#"
type Value = union<uint8; 0: ref<int32, unique>, 1: int32>

function test(v0: Value): void {
b0(v0: Value):
    v1: ref<int32, unique> = field.get v0, 0
    v2: int32 = field.get v0, 1
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_use_after_move(&errors);
}

#[test]
fn test_reject_partial_move_of_drop_type() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_partial_move_of_drop_type(&errors);
}

#[test]
fn test_reject_move_while_borrowed_through_block_parameter() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_change_of_borrowed_place(&errors);
}

#[test]
fn test_reject_local_set_while_borrowed() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_change_of_borrowed_place(&errors);
}

#[test]
fn test_reject_store_through_readonly_borrow() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
b0(v0: ref<int32, borrowed, readonly>, v1: int32):
    store v0, v1
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_readonly_write(&errors);
}

#[test]
fn test_reject_store_while_exclusive_borrow_is_live() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_change_of_borrowed_place(&errors);
}

#[test]
fn test_allow_store_through_own_exclusive_borrow() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_store_through_live_borrow() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed>, v1: int32): void {
b0(v0: ref<int32, borrowed>, v1: int32):
    store v0, v1
    v2: int32 = load v0
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_use_after_call_move() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_use_after_move(&errors);
}

#[test]
fn test_reject_use_after_struct_move() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_use_after_move(&errors);
}

#[test]
fn test_ignore_raw_free_for_moves() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    raw.free v0
    raw.free v0
    return
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_frame_return() {
    let mut program = VerifyProgram::new(
        r#"
type Box {
    int32;
}

function test(): ref<int32, borrowed> {
b0:
    v0: ref<Box, raw, space(stack)> = stack.alloc Box
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
    );

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_allow_parameter_return_when_declared() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_infer_parameter_return_lifetime() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_infer_managed_return_lifetime() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_infer_managed_slice_return_lifetime() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: slice<int32, managed>): ref<int32, borrowed, readonly> {
b0(v0: slice<int32, managed>):
    v1: int64 = 0int64
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    return v2
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_managed_borrow_across_yield() {
    let mut program = VerifyProgram::new(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): int32 {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    yield v0, b1(v1)
b1(v2: ref<User, managed>, v3: ref<int32, borrowed, readonly>):
    v4: int32 = load v3
    return v4
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_unique_slice_borrow_return() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: slice<int32, unique>): ref<int32, borrowed> {
b0(v0: slice<int32, unique>):
    v1: int64 = 0int64
    v2: ref<int32, borrowed> = element.address v0, v1
    return v2
}"#,
    );

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_unique_borrow_return() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_wrong_managed_parameter_lifetime_return() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_managed_return_as_static() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_aggregate_borrow_return_as_static() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_union_borrow_return_as_static() {
    let mut program = VerifyProgram::new(
        r#"
type Value = union<uint8; 0: ref<int32, borrowed>, 1: int32>

function test(v0: Value): ref<int32, borrowed, lifetime(static)> {
b0(v0: Value):
    v1: ref<int32, borrowed> = field.get v0, 1
    return v1
}"#,
    );

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_allow_frame_borrow_across_yield() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_mutable_parameter_borrow_across_yield() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed>): int32 {
entry0(v0: ref<int32, borrowed>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed>):
    v2: int32 = load v1
    return v2
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_readonly_parameter_borrow_across_yield() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed, readonly>): int32 {
entry0(v0: ref<int32, borrowed, readonly>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed, readonly>):
    v2: int32 = load v1
    return v2
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_exclusive_borrow_across_yield() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_exclusive_loan_across_suspension(&errors);
}

#[test]
fn test_reject_exclusive_parameter_across_yield() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed, exclusive>): int32 {
entry0(v0: ref<int32, borrowed, exclusive>):
    yield v0, block1(v0)
block1(v1: ref<int32, borrowed, exclusive>):
    v2: int32 = load v1
    return v2
}"#,
    );

    let errors = program.run_ownership();

    assert_exclusive_loan_across_suspension(&errors);
}

#[test]
fn test_allow_borrow_before_yield() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_allow_static_borrow_return() {
    let mut program = VerifyProgram::new(
        r#"
global value: int32, readonly = 1int32
function test(): ref<int32, borrowed, lifetime(static)> {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: ref<int32, borrowed, readonly, lifetime(static)> = cast.bit v0 -> ref<int32, borrowed, readonly, lifetime(static)>
    return v1
}"#,
    );

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_reject_wrong_parameter_lifetime_return() {
    let mut program = VerifyProgram::new(
        r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v1
}"#,
    );

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}

#[test]
fn test_reject_maybe_moved_after_join() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_maybe_use_after_move(&errors);
}

#[test]
fn test_carry_lifetime_through_block_parameter() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_no_errors(&errors);
}

#[test]
fn test_merge_lifetimes_at_join() {
    let mut program = VerifyProgram::new(
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

    let errors = program.run_ownership();

    assert_borrow_outlives_origin(&errors);
}
