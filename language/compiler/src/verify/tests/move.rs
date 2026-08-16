use crate::tests::TestProgram;

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

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:11:5
   │
 8 │ entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
 9 │     v2: Pair = aggregate (v0, v1)
10 │     v3: ref<int32, unique, mutable> = field.get v2, 0
   │     ------------------------------------------------- value moved here
11 │     v4: ref<int32, unique, mutable> = field.get v2, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     v5: ref<int32, unique, mutable> = field.get v2, 1
13 │     return
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
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

    program.assert_verified();
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

    program.assert_verified();
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

    program.assert_verified();
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

    program.assert_verified();
}

#[test]
fn test_allow_aggregate_move_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair): void {
entry(v0: Pair):
    jump next(v0)

next(v1: Pair):
    v2: ref<int32, unique, mutable> = field.get v1, 0
    v3: ref<int32, unique, mutable> = field.get v1, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_return() {
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

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_call() {
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
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_sibling_use_after_partial_move() {
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

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v1): (ref<int32, unique, mutable>) => void
    v2: ref<int32, unique, mutable> = field.get v0, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_reinitialization_after_partial_move() {
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

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v2: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v2): (ref<int32, unique, mutable>) => void
    v3: Pair = field.set v0, 0, v1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_across_branch() {
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
    branch v2 => b1 | b2

b1:
    return

b2:
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_panic() {
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

    program.assert_verified();
}

#[test]
fn test_reject_variant_use_after_payload_move() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0
    v2: uint8 = variant.tag v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
 ──▶ <test.dsm>:7:5
  │
4 │ function test(v0: Value): void {
5 │ entry(v0: Value):
6 │     v1: ref<int32, unique, mutable> = variant.payload v0, 0
  │     ------------------------------------------------------- value moved here
7 │     v2: uint8 = variant.tag v0
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^
8 │     return
9 │ }
  │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}
