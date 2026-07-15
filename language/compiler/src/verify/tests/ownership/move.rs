use crate::tests::TestProgram;

#[test]
fn test_reject_move_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function consume(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    call consume(v0)
    v2: int32 = load v1
    return
}
"#,
    );

    program.assert_error_invalidation_of_borrowed_place();
}

#[test]
fn test_reject_projected_move_use_after_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 0
    v5: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_allow_complete_struct_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_complete_tuple_decomposition() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: { ref<int32, unique, mutable>, ref<int32, unique, mutable> } = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_complete_array_decomposition() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: [ref<int32, unique, mutable>; 2] = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = element.get v2, 0
    v4: ref<int32, unique, mutable> = element.get v2, 1
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_complete_nested_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v6: ref<int32, unique, mutable> = field.get v4, 1
    v7: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_incomplete_decomposition_before_return() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    return
}
"#,
    );

    program.assert_error_partial_move();
}

#[test]
fn test_reject_incomplete_decomposition_before_call() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    call consume(v3)
    return
}
"#,
    );

    program.assert_error_partial_move();
}

#[test]
fn test_reject_incomplete_decomposition_before_branch() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    v3: Pair = aggregate (v0, v1)
    v4: ref<int32, unique, mutable> = field.get v3, 0
    branch v2, b1, b2

b1:
    return

b2:
    return
}
"#,
    );

    program.assert_error_partial_move();
}

#[test]
fn test_reject_incomplete_decomposition_before_yield() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: int32): int32 {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: int32):
    v3: Pair = aggregate (v0, v1)
    v4: ref<int32, unique, mutable> = field.get v3, 0
    yield v2 => b1

b1(v5: int32):
    return v5
}
"#,
    );

    program.assert_error_partial_move();
}

#[test]
fn test_reject_incomplete_decomposition_before_panic() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, managed, readonly>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, managed, readonly>):
    v3: Pair = aggregate (v0, v1)
    v4: ref<int32, unique, mutable> = field.get v3, 0
    panic v2
}
"#,
    );

    program.assert_error_partial_move();
}

#[test]
fn test_reject_variant_use_after_payload_move() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = field.get v0, 1
    v2: int32 = field.get v0, 1
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_reject_move_out_of_drop() {
    let mut program = TestProgram::mir(
        r#"
type Row {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

external function dropRow(ref<Row, borrowed, exclusive>): void

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Row = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    return
}
"#,
    );
    program.mark_drop_hook("Row", "dropRow");

    program.assert_error_move_out_of_drop();
}

#[test]
fn test_reject_move_out_of_variant_with_drop() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; };

external function dropValue(ref<Value, borrowed, exclusive>): void

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = field.get v0, 1
    return
}
"#,
    );
    program.mark_drop_hook("Value", "dropValue");

    program.assert_error_move_out_of_drop();
}

#[test]
fn test_reject_move_while_borrowed_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function consume(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}

function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
entry(v0: ref<Box, unique, mutable>, v1: boolean):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    branch v1, b1(v2), b2

b1(v3: ref<int32, borrowed, mutable>):
    call consume(v0)
    v4: int32 = load v3
    return

b2:
    return
}
"#,
    );

    program.assert_error_invalidation_of_borrowed_place();
}

#[test]
fn test_reject_use_after_call_move() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    call consume(v0)
    v1: int32 = load v0
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_reject_use_after_struct_move() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_reject_use_after_new_complete() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): void {
entry:
    v0: uninit<ref<Box, managed, mutable>> = new.uninit Box
    v1: ref<Box, managed, mutable> = new.complete v0
    v2: ref<Box, managed, mutable> = new.complete v0
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_ignore_raw_free_for_moves() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, raw, mutable>): void {
entry(v0: ref<int32, raw, mutable>):
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_maybe_moved_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1, b1, b2

b1:
    call consume(v0)
    jump b3

b2:
    jump b3

b3:
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_error_maybe_use_after_move();
}
