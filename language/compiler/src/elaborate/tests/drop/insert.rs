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
    call later(): () => void
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
    call later(): () => void
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
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2): (ref<int32, unique, mutable>) => void
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
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2): (ref<int32, unique, mutable>) => void
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
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
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
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
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
    invoke callee(): () => int32 => b1 | cleanup

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
    invoke callee(): () => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
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
    invoke callee(): () => int32 => b1 | cleanup

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
    invoke callee(): () => int32 => b1 | b2

b1(v1: int32):
    v2: int32 = load v0
    free v0
    return v2

b2:
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
    yield v1 => b1(v0) | b1(v0) | cleanup

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
    yield v1 => b1(v0) | b1(v0) | b2

b1(v2: int32, v3: ref<int32, unique, mutable>):
    free v3
    return v2

b2:
    free v0
    unwind.resume
}
"#,
    );
}

#[test]
fn test_insert_drop_after_complete_struct_decomposition() {
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
    v4: ref<int32, unique, mutable> = field.get v2, 1
    call consume(v3): (ref<int32, unique, mutable>) => void
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
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    free v4
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}

function Pair.destruct.frame(v0: ref<Pair, borrowed, exclusive, frame>): void {
entry(v0: ref<Pair, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<int32, unique, mutable> = load v3
    free v4
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_after_complete_nested_decomposition() {
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
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable> = field.get v4, 1
    v6: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    call consume(v6): (ref<int32, unique, mutable>) => void
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
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable> = field.get v4, 1
    free v7
    v6: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    free v8
    call consume(v6): (ref<int32, unique, mutable>) => void
    return
}

function Pair.destruct.frame(v0: ref<Pair, borrowed, exclusive, frame>): void {
entry(v0: ref<Pair, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<int32, unique, mutable> = load v3
    free v4
    return
}

function Outer.destruct.frame(v0: ref<Outer, borrowed, exclusive, frame>): void {
entry(v0: ref<Outer, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<Pair, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v3, 1
    v5: ref<int32, unique, mutable> = load v4
    free v5
    v6: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v3, 0
    v7: ref<int32, unique, mutable> = load v6
    free v7
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
    v1: ref<int32, unique, mutable> = field.get v0, 1
    return
}
"#,
    );

    program.assert_drop_mir(
        r#"
type Value = variant<uint8, ref<int32, unique, mutable>> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = field.get v0, 1
    free v1
    return
}

function Value.destruct.frame(v0: ref<Value, borrowed, exclusive, frame>): void {
entry(v0: ref<Value, borrowed, exclusive, frame>):
    v1: ref<uint8, borrowed, exclusive, frame> = field.address v0, 0
    v2: uint8 = load v1
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v4: uint8 = 0
    v5: boolean = int.eq v2, v4
    branch v5 => b1 | b2

b1:
    v6: ref<int32, unique, mutable> = load v3
    free v6
    jump b2

b2:
    return
}
"#,
    );
}
