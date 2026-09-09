use crate::tests::{TestProgram, TestSession};

#[test]
fn test_reject_select_of_move_only_values() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: Box, v1: Box, v2: boolean): Box {
entry(v0: Box, v1: Box, v2: boolean):
    v3: Box = select v2, v0, v1
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[select-of-move-only-value]: invalid MIR: select operands must implement Copy
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: Box, v1: Box, v2: boolean): Box {
 7 │ entry(v0: Box, v1: Box, v2: boolean):
 8 │     v3: Box = select v2, v0, v1
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return v3
10 │ }
   │

for more information about an error, run `destack explain select-of-move-only-value`
"#,
    );
}

#[test]
fn test_reject_move_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function consume(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    return
}

function test(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    v1: ref<int32, borrowed, 'frame, mutable, local> = field.address v0, 0
    call consume(v0): (ref<Box, unique, mutable, local>) => void
    v2: int32 = load v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:14:5
   │
11 │ function test(v0: ref<Box, unique, mutable, local>): void {
12 │ entry(v0: ref<Box, unique, mutable, local>):
13 │     v1: ref<int32, borrowed, 'frame, mutable, local> = field.address v0, 0
   │     ---------------------------------------------------------------------- borrow starts here
14 │     call consume(v0): (ref<Box, unique, mutable, local>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
15 │     v2: int32 = load v1
16 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

#[test]
fn test_reject_move_out_through_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, local> = field.address v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
  ──▶ <test.dsm>:9:5
   │
 7 │ entry(v0: ref<Box, borrowed, 'a, mutable, local>):
 8 │     v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, local> = field.address v0, 0
 9 │     v2: ref<int32, unique, mutable, local> = load v1
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain move-out-of-reference`
"#,
    );
}

/// A move-only value may be stored through a mutable borrow, the old value dropping deferred.
#[test]
fn test_allow_a_move_only_store_through_a_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<int32, unique, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<int32, unique, mutable, local>):
    v2: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, local> = field.address v0, 0
    store v2, v1
    return
}
"#,
    );

    program.assert_verified();
}

/// A store over a move-only value through an exclusive borrow is allowed; elaboration drops the old value.
#[test]
fn test_allow_a_move_only_store_through_an_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<int32, unique, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<int32, unique, mutable, local>):
    v2: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, local> = field.address v0, 0
    store v2, v1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_move_out_of_drop() {
    let mut program = TestProgram::mir(
        r#"
type Row {
    left: ref<int32, unique, mutable, local>;
    right: ref<int32, unique, mutable, local>;
}

external function dropRow<'a>(ref<Row, borrowed, 'a, mutable, local>): void

function test(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>):
    v2: Row = aggregate (v0, v1)
    v3: ref<int32, unique, mutable, local> = field.get v2, 0
    return
}
"#,
    );
    program.mark_drop_hook("Row", "dropRow");

    program.assert_verify_errors(
        r#"
error[move-out-of-drop]: cannot move out of a value that implements Drop
  ──▶ <test.dsm>:12:5
   │
10 │ entry(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>):
11 │     v2: Row = aggregate (v0, v1)
12 │     v3: ref<int32, unique, mutable, local> = field.get v2, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
13 │     return
14 │ }
   │

for more information about an error, run `destack explain move-out-of-drop`
"#,
    );
}

#[test]
fn test_reject_move_out_of_variant_with_drop() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable, local>; };

external function dropValue<'a>(ref<Value, borrowed, 'a, mutable, local>): void

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable, local> = variant.payload v0, 0
    return
}
"#,
    );
    program.mark_drop_hook("Value", "dropValue");

    program.assert_verify_errors(
        r#"
error[move-out-of-drop]: cannot move out of a value that implements Drop
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: Value): void {
 7 │ entry(v0: Value):
 8 │     v1: ref<int32, unique, mutable, local> = variant.payload v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain move-out-of-drop`
"#,
    );
}

/// A load through a unique reference takes its pointee, leaving the allocation to free.
#[test]
fn test_allow_taking_a_unique_pointee_before_its_free() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): Box {
entry(v0: ref<Box, unique, mutable, local>):
    v1: Box = load v0
    release v0
    return v1
}
"#,
    );

    program.assert_verified();
}

/// A second load of a taken unique pointee is a use after move.
#[test]
fn test_reject_a_second_load_of_a_taken_unique_pointee() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): Box {
entry(v0: ref<Box, unique, mutable, local>):
    v1: Box = load v0
    v2: Box = load v0
    release v0
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(v0: ref<Box, unique, mutable, local>): Box {
 7 │ entry(v0: ref<Box, unique, mutable, local>):
 8 │     v1: Box = load v0
   │     ----------------- value moved here
 9 │     v2: Box = load v0
   │     ^^^^^^^^^^^^^^^^^
10 │     release v0
11 │     return v2
   │

error[use-after-move]: use of moved value
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(v0: ref<Box, unique, mutable, local>): Box {
 7 │ entry(v0: ref<Box, unique, mutable, local>):
 8 │     v1: Box = load v0
   │     ----------------- value moved here
 9 │     v2: Box = load v0
   │     ^^^^^^^^^^^^^^^^^
10 │     release v0
11 │     return v2
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

/// A free of a freed reference is a use after move.
#[test]
fn test_reject_a_free_of_a_freed_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): Box {
entry(v0: ref<Box, unique, mutable, local>):
    v1: Box = load v0
    release v0
    release v0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:10:5
   │
 7 │ entry(v0: ref<Box, unique, mutable, local>):
 8 │     v1: Box = load v0
 9 │     release v0
   │     ---------- value moved here
10 │     release v0
   │     ^^^^^^^^^^
11 │     return v1
12 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

/// A move-only load through a managed reference stays a move out of a reference.
#[test]
fn test_reject_a_move_out_through_a_managed_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, managed, mutable, local>): Box {
entry(v0: ref<Box, managed, mutable, local>):
    v1: Box = load v0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, managed, mutable, local>): Box {
 7 │ entry(v0: ref<Box, managed, mutable, local>):
 8 │     v1: Box = load v0
   │     ^^^^^^^^^^^^^^^^^
 9 │     return v1
10 │ }
   │

for more information about an error, run `destack explain move-out-of-reference`
"#,
    );
}

/// A destructuring move invalidates the borrow of its value.
#[test]
fn test_reject_a_destructuring_move_while_the_value_is_borrowed() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

struct S {
    x: Token;
}

function use(s: &readonly S): void {}

export function main(): void {
    const a = S { x: Token { id: 1 } };
    const pb = &readonly a;
    const S { x } = a;
    use(pb);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=17 column=21 span="a" line_source="const S { x } = a;"
/// @diagnostic.related line=16 column=16 span="&readonly a" line_source="const pb = &readonly a;" message="borrow starts here"
"#);
}

/// A move invalidates the borrow of its value.
#[test]
fn test_reject_a_move_while_the_value_is_borrowed() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

function take(token: Token): void {}

function useRef(token: &readonly Token): void {}

export function boxImm(): void {
    const v = Token { id: 3 };
    const w = &readonly v;
    take(v);
    useRef(w);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=15 column=10 span="v" line_source="take(v);"
/// @diagnostic.related line=14 column=15 span="&readonly v" line_source="const w = &readonly v;" message="borrow starts here"
"#);
}
