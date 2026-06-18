use crate::tests::TestProgram;

#[test]
fn test_insert_drop_after_last_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    return v1
}
"#,
    );

    program.assert_drop_mir(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    free v0
    return v1
}
"#,
    );
}

#[test]
fn test_insert_drop_before_later_unrelated_use() {
    let mut program = TestProgram::mir(
        r#"
function later(): void {
entry:
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    call later()
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
function later(): void {
entry:
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    free v0
    call later()
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_after_last_interior_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_drop_mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    free v0
    return v2
}
"#,
    );
}

#[test]
fn test_insert_drop_for_unused_owned_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_unique_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    free v1
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_unconsumed_branch() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1, b1(v0), b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2)
    return

b2(v3: ref<int32, unique, mutable>):
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1, b1(v0), b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2)
    return

b2(v3: ref<int32, unique, mutable>):
    free v3
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_branch_path_without_owned_use() {
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
    return

b2:
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1, b1, b2

b1:
    call consume(v0)
    return

b2:
    free v0
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_call_unwind_path() {
    let mut program = TestProgram::mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    call callee() => b1 | cleanup

b1(v1: int32):
    return v1

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_drop_mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    free v0
    call callee() => b1 | cleanup

b1(v1: int32):
    return v1

cleanup:
    unwind.resume
}
"#,
    );
}

#[test]
fn test_insert_drop_on_call_cleanup_when_value_remains_live() {
    let mut program = TestProgram::mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    call callee() => b1 | cleanup

b1(v1: int32):
    v2: int32 = load v0
    return v2

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_drop_mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    call callee() => b1 | cleanup

b1(v1: int32):
    v2: int32 = load v0
    free v0
    return v2

cleanup:
    free v0
    unwind.resume
}
"#,
    );
}

#[test]
fn test_insert_drop_on_yield_unwind_path() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: int32): int32 {
entry(v0: ref<int32, unique, mutable>, v1: int32):
    yield v1 => b1(v0) | cleanup

b1(v2: int32, v3: ref<int32, unique, mutable>):
    return v2

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_drop_mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: int32): int32 {
entry(v0: ref<int32, unique, mutable>, v1: int32):
    yield v1 => b1(v0) | cleanup

b1(v2: int32, v3: ref<int32, unique, mutable>):
    free v3
    return v2

cleanup:
    free v0
    unwind.resume
}
"#,
    );
}

#[test]
fn test_insert_drop_for_remaining_struct_field_after_partial_move() {
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
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    call consume(v3)
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    free v4
    call consume(v3)
    return
}

function Pair.drop(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    v2: ref<int32, unique, mutable> = field.get v0, 1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_nested_remaining_fields_after_partial_move() {
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

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = struct Pair (v0, v1)
    v4: Outer = struct Outer (v3, v2)
    v5: Pair = field.get v4, 0
    v6: ref<int32, unique, mutable> = field.get v5, 0
    call consume(v6)
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = struct Pair (v0, v1)
    v4: Outer = struct Outer (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable> = field.get v4, 1
    free v7
    v6: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    free v8
    call consume(v6)
    return
}

function Pair.drop(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    v2: ref<int32, unique, mutable> = field.get v0, 1
    free v2
    return
}

function Outer.drop(v0: Outer): void {
entry(v0: Outer):
    v1: Pair = field.get v0, 0
    call Pair.drop(v1)
    v2: ref<int32, unique, mutable> = field.get v0, 1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_variant_payload_move_consumes_variant() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0uint8
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0uint8
    free v1
    return
}

function Value.drop(v0: Value): void {
entry(v0: Value):
    v1: uint8 = variant.tag v0
    check variant.tag v1, 0uint8 => b1, b2

b1:
    v2: ref<int32, unique, mutable> = variant.payload v0, 0uint8
    free v2
    jump b2

b2:
    return
}
"#,
    );
}
