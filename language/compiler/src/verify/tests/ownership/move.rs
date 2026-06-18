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
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 0
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_allow_disjoint_field_use_after_projected_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_variant_use_after_payload_move() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0uint8
    v2: int32 = variant.payload v0, 1uint8
    return
}
"#,
    );

    program.assert_error_use_after_move();
}

#[test]
fn test_reject_partial_move_of_custom_drop() {
    let mut program = TestProgram::mir(
        r#"
type Row {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

external function dropRow(Row): void

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Row = struct Row (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    return
}
"#,
    );
    program.mark_function_drop("Row", "dropRow");

    program.assert_error_partial_move_of_custom_drop();
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
    v1: Box = struct Box (v0)
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
