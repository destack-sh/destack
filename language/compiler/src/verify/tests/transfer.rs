use crate::tests::TestProgram;

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

function consume(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    call consume(v0): (ref<Box, unique, mutable>) => void
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
11 │ function test(v0: ref<Box, unique, mutable>): void {
12 │ entry(v0: ref<Box, unique, mutable>):
13 │     v1: ref<int32, borrowed, mutable> = field.address v0, 0
   │     ------------------------------------------------------- borrow starts here
14 │     call consume(v0): (ref<Box, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
  ──▶ <test.dsm>:9:5
   │
 7 │ entry(v0: ref<Box, borrowed, exclusive>):
 8 │     v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
 9 │     v2: ref<int32, unique, mutable> = load v1
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain move-out-of-reference`
"#,
    );
}

#[test]
fn test_reject_move_only_store_over_initialized_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, borrowed, exclusive>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<Box, borrowed, exclusive>, v1: ref<int32, unique, mutable>):
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[overwrite-of-move-only-place]: cannot overwrite move-only storage without taking its value
  ──▶ <test.dsm>:9:5
   │
 7 │ entry(v0: ref<Box, borrowed, exclusive>, v1: ref<int32, unique, mutable>):
 8 │     v2: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
 9 │     store v2, v1
   │     ^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain overwrite-of-move-only-place`
"#,
    );
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

    program.assert_verify_errors(
        r#"
error[move-out-of-drop]: cannot move out of a value that implements Drop
  ──▶ <test.dsm>:12:5
   │
10 │ entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
11 │     v2: Row = aggregate (v0, v1)
12 │     v3: ref<int32, unique, mutable> = field.get v2, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable>; };

external function dropValue(ref<Value, borrowed, exclusive>): void

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0
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
 8 │     v1: ref<int32, unique, mutable> = variant.payload v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain move-out-of-drop`
"#,
    );
}
