use crate::tests::TestProgram;

#[test]
fn test_reject_exclusive_overlap() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>): void {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:10:5
   │
 7 │ function test(v0: ref<Box, borrowed, mutable>): void {
 8 │ entry(v0: ref<Box, borrowed, mutable>):
 9 │     v1: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     --------------------------------------------------------- borrow starts here
10 │     v2: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     v3: int32 = load v1
12 │     v4: int32 = load v2
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_allow_aliasable_mutable_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>): void {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_exclusive_borrows_from_distinct_exclusive_parameters() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, exclusive>, v1: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>, v1: ref<Box, borrowed, exclusive>):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: ref<int32, borrowed, exclusive> = field.address v1, 0
    v4: int32 = load v2
    v5: int32 = load v3
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_exclusive_disjoint_fields() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test(v0: ref<Pair, borrowed, mutable>): void {
entry(v0: ref<Pair, borrowed, mutable>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 1
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_exclusive_dynamic_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4], v1: usize, v2: usize): void {
entry(v0: [int32; 4], v1: usize, v2: usize):
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
 ──▶ <test.dsm>:5:5
  │
2 │ function test(v0: [int32; 4], v1: usize, v2: usize): void {
3 │ entry(v0: [int32; 4], v1: usize, v2: usize):
4 │     v3: ref<int32, borrowed, exclusive> = element.address v0, v1
  │     ------------------------------------------------------------ borrow starts here
5 │     v4: ref<int32, borrowed, exclusive> = element.address v0, v2
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     v5: int32 = load v3
7 │     v6: int32 = load v4
  │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_allow_exclusive_constant_element_disjoint() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4]): void {
entry(v0: [int32; 4]):
    v1: uint64 = 0
    v2: uint64 = 1
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_exclusive_borrowed_slice_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed, mutable>, v1: usize, v2: usize): void {
entry(v0: slice<int32, borrowed, mutable>, v1: usize, v2: usize):
    v3: ref<int32, borrowed, exclusive> = element.address v0, v1
    v4: ref<int32, borrowed, exclusive> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
 ──▶ <test.dsm>:5:5
  │
2 │ function test(v0: slice<int32, borrowed, mutable>, v1: usize, v2: usize): void {
3 │ entry(v0: slice<int32, borrowed, mutable>, v1: usize, v2: usize):
4 │     v3: ref<int32, borrowed, exclusive> = element.address v0, v1
  │     ------------------------------------------------------------ borrow starts here
5 │     v4: ref<int32, borrowed, exclusive> = element.address v0, v2
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     v5: int32 = load v3
7 │     v6: int32 = load v4
  │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_allow_exclusive_disjoint_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed, mutable>): void {
entry(v0: slice<int32, borrowed, mutable>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: slice<int32, borrowed, exclusive> = slice.view v0, v1, v2
    v4: slice<int32, borrowed, exclusive> = slice.view v0, v2, v2
    v5: ref<int32, borrowed, exclusive> = element.address v3, v1
    v6: ref<int32, borrowed, exclusive> = element.address v4, v1
    v7: int32 = load v5
    v8: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_exclusive_overlapping_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, borrowed, mutable>): void {
entry(v0: slice<int32, borrowed, mutable>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: uint64 = 1
    v4: slice<int32, borrowed, exclusive> = slice.view v0, v1, v2
    v5: slice<int32, borrowed, exclusive> = slice.view v0, v3, v2
    v6: ref<int32, borrowed, exclusive> = element.address v4, v1
    v7: ref<int32, borrowed, exclusive> = element.address v5, v1
    v8: int32 = load v6
    v9: int32 = load v7
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:8:5
   │
 5 │     v2: uint64 = 2
 6 │     v3: uint64 = 1
 7 │     v4: slice<int32, borrowed, exclusive> = slice.view v0, v1, v2
   │     ------------------------------------------------------------- borrow starts here
 8 │     v5: slice<int32, borrowed, exclusive> = slice.view v0, v3, v2
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     v6: ref<int32, borrowed, exclusive> = element.address v4, v1
10 │     v7: ref<int32, borrowed, exclusive> = element.address v5, v1
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_reject_exclusive_after_readonly_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>): void {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(v0: ref<Box, borrowed, mutable>): void {
 7 │ entry(v0: ref<Box, borrowed, mutable>):
 8 │     v1: ref<int32, borrowed, readonly> = field.address v0, 0
   │     -------------------------------------------------------- borrow starts here
 9 │     v2: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     v3: int32 = load v1
11 │     v4: int32 = load v2
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_allow_exclusive_after_last_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>): void {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, exclusive> = field.address v0, 0
    v4: int32 = load v3
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_change_after_borrow_returns_on_other_branch() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'L>(v0: ref<Box, borrowed, 'L, mutable>, v1: boolean, v2: int32): ref<int32, borrowed, 'L, readonly> {
entry(v0: ref<Box, borrowed, 'L, mutable>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, 'L, readonly> = field.address v0, 0
    branch v1 => found(v3) | missing

found(v4: ref<int32, borrowed, 'L, readonly>):
    return v4

missing:
    v5: ref<int32, borrowed, 'L, mutable> = field.address v0, 0
    store v5, v2
    return v5
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_change_after_loop_carried_borrow_is_replaced() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test(v0: ref<Pair, borrowed, mutable>, v1: boolean, v2: int32): void {
    local l0: ref<int32, borrowed, readonly>

entry(v0: ref<Pair, borrowed, mutable>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, readonly> = field.address v0, 0
    local.set l0, v3
    jump next

next:
    branch v1 => replace | done

replace:
    v4: ref<int32, borrowed, readonly> = field.address v0, 1
    local.set l0, v4
    v5: ref<int32, borrowed, mutable> = field.address v0, 0
    store v5, v2
    jump next

done:
    v6: ref<int32, borrowed, readonly> = local.get l0
    v7: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}
