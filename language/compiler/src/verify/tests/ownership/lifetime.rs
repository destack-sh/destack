use crate::tests::{TestProgram, TestSession};

#[test]
fn test_reject_frame_return() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): ref<int32, borrowed, mutable> {
    local l0: Box
entry:
    v0: ref<Box, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_allow_parameter_return_when_declared() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0>(v0: ref<int32, borrowed, 'L0, mutable>): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>):
    return v0
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_parameter_return_without_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, mutable>): ref<int32, borrowed, mutable> {
entry(v0: ref<int32, borrowed, mutable>):
    return v0
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_allow_managed_field_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'L0>(v0: ref<User, managed, 'L0, mutable>): ref<int32, borrowed, 'L0, readonly> {
entry(v0: ref<User, managed, 'L0, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_managed_slice_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0>(v0: slice<int32, managed, 'L0, mutable>): ref<int32, borrowed, 'L0, readonly> {
entry(v0: slice<int32, managed, 'L0, mutable>):
    v1: int64 = 0
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    return v2
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'L0>(v0: ref<User, managed, 'L0, mutable>, v1: boolean): ref<int32, borrowed, 'L0, readonly> {
entry(v0: ref<User, managed, 'L0, mutable>, v1: boolean):
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    branch v1 => b1(v2) | b2(v2)

b1(v3: ref<int32, borrowed, 'L0, readonly>):
    return v3

b2(v4: ref<int32, borrowed, 'L0, readonly>):
    return v4
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0>(v0: slice<int32, managed, 'L0, mutable>, v1: boolean): ref<int32, borrowed, 'L0, readonly> {
entry(v0: slice<int32, managed, 'L0, mutable>, v1: boolean):
    v2: int64 = 0
    v3: ref<int32, borrowed, readonly> = element.address v0, v2
    branch v1 => b1(v3) | b2(v3)

b1(v4: ref<int32, borrowed, 'L0, readonly>):
    return v4

b2(v5: ref<int32, borrowed, 'L0, readonly>):
    return v5
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_unique_slice_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, unique, mutable>): ref<int32, borrowed, mutable> {
entry(v0: slice<int32, unique, mutable>):
    v1: int64 = 0
    v2: ref<int32, borrowed, mutable> = element.address v0, v1
    return v2
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_unique_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, unique, mutable>): ref<int32, borrowed, mutable> {
entry(v0: ref<User, unique, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_wrong_managed_parameter_lifetime_return() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'L0, 'L1>(v0: ref<User, managed, 'L0, mutable>, v1: ref<User, managed, 'L1, mutable>): ref<int32, borrowed, 'L0, readonly> {
entry(v0: ref<User, managed, 'L0, mutable>, v1: ref<User, managed, 'L1, mutable>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    return v2
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_managed_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable>): ref<int32, borrowed, 'static, readonly> {
entry(v0: ref<User, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_aggregate_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, borrowed, mutable>;
}

function test(v0: Box): ref<int32, borrowed, 'static, mutable> {
entry(v0: Box):
    v1: ref<int32, borrowed, mutable> = field.get v0, 0
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_allow_aggregate_field_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, readonly>, v1: ref<int32, borrowed, 'L1, readonly>, v2: Pair<'L0, 'L1>): ref<int32, borrowed, 'L0, readonly> {
entry(v0: ref<int32, borrowed, 'L0, readonly>, v1: ref<int32, borrowed, 'L1, readonly>, v2: Pair<'L0, 'L1>):
    v3: ref<int32, borrowed, 'L0, readonly> = field.get v2, 0
    return v3
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_aggregate_field_return_with_wrong_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, readonly>, v1: ref<int32, borrowed, 'L1, readonly>, v2: Pair<'L0, 'L1>): ref<int32, borrowed, 'L0, readonly> {
entry(v0: ref<int32, borrowed, 'L0, readonly>, v1: ref<int32, borrowed, 'L1, readonly>, v2: Pair<'L0, 'L1>):
    v3: ref<int32, borrowed, 'L1, readonly> = field.get v2, 1
    return v3
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_allow_aggregate_return_with_distinct_path_lifetimes() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: Pair<'L0, 'L1>): Pair<'L0, 'L1> {
entry(v0: Pair<'L0, 'L1>):
    return v0
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_aggregate_return_with_swapped_path_lifetimes() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: Pair<'L0, 'L1>): Pair<'L1, 'L0> {
entry(v0: Pair<'L0, 'L1>):
    return v0
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_variant_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, borrowed, mutable>; 1uint8 = int32; };

function test(v0: Value): ref<int32, borrowed, 'static, mutable> {
entry(v0: Value):
    v1: ref<int32, borrowed, mutable> = field.get v0, 1
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_allow_static_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
readonly global value: int32 = 1

function test(): ref<int32, borrowed, 'static, mutable> {
entry:
    v0: ref<int32, borrowed, readonly> = global.address value
    v1: ref<int32, borrowed, 'static, readonly> = cast.bit v0 -> ref<int32, borrowed, 'static, readonly>
    return v1
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_wrong_parameter_lifetime_return() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>):
    return v1
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_carry_lifetime_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0>(v0: ref<int32, borrowed, 'L0, mutable>, v1: boolean): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, borrowed, 'L0, mutable>):
    return v2

b2(v3: ref<int32, borrowed, 'L0, mutable>):
    return v3
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_carry_aggregate_path_lifetimes_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: Pair<'L0, 'L1>, v1: boolean): ref<int32, borrowed, 'L1, readonly> {
entry(v0: Pair<'L0, 'L1>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: Pair<'L0, 'L1>):
    jump b3(v2)

b2(v3: Pair<'L0, 'L1>):
    jump b3(v3)

b3(v4: Pair<'L0, 'L1>):
    v5: ref<int32, borrowed, 'L1, readonly> = field.get v4, 1
    return v5
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_merge_lifetimes_at_join() {
    let mut program = TestProgram::mir(
        r#"
function test<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>, v2: boolean): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>, v2: boolean):
    branch v2 => b1 | b2

b1:
    jump b3(v0)

b2:
    jump b3(v1)

b3(v3: ref<int32, borrowed, mutable>):
    return v3
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_widened_result_lifetimes_from_source() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function pick<'a, 'b, 'c>(a: &'a Node, b: &'b Node): &'c Node {
    return a;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=borrow-outlives-origin message="borrow does not live long enough"
/// @diagnostic.label file="main.ds"
"#,
    );
}

#[test]
fn test_allow_declared_outlives_result_from_source() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function pass<'a, 'c>(a: &'a Node): &'c Node where 'a: 'c {
    return a;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

#[test]
fn test_allow_call_arguments_satisfying_outlives_rows() {
    let mut program = TestProgram::mir(
        r#"
function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable>): void {
entry(v0: ref<int32, borrowed, 'L, mutable>):
    call callee(v0, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_call_arguments_violating_outlives_rows() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable>): void {
    local l0: Box

entry(v0: ref<int32, borrowed, 'L, mutable>):
    v1: ref<Box, borrowed, mutable, frame> = local.address l0
    v2: ref<int32, borrowed, mutable> = field.address v1, 0
    call callee(v2, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_error_borrow_outlives_origin();
}

#[test]
fn test_reject_disjunctive_lifetime_bounds() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function pick<'a, 'b, 'c>(a: &'a Node, b: &'b Node): &'c Node where 'c: 'a | 'b {
    return a;
}
"#,
    );

    session.assert_dir_checked_diagnostics("main.ds", r#"
/// @diagnostic.error id=disjunctive-lifetime-bound message="a lifetime bound must name one lifetime, not a union"
/// @diagnostic.label line=6 column=69 span="'c" line_source="function pick<'a, 'b, 'c>(a: &'a Node, b: &'b Node): &'c Node where 'c: 'a | 'b {"
"#);
}

#[test]
fn test_allow_static_return_at_slot_result() {
    let mut program = TestProgram::mir(
        r#"
readonly global value: int32 = 1

function test<'L>(): ref<int32, borrowed, 'L, readonly> {
entry:
    v0: ref<int32, borrowed, readonly> = global.address value
    return v0
}
"#,
    );

    program.assert_no_ownership_errors();
}
