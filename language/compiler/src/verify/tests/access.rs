use crate::tests::TestProgram;

#[test]
fn test_reject_local_set_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    local.set l0, v1
    v3: int32 = load v2
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:8:5
   │
 5 │ entry(v0: int32, v1: int32):
 6 │     local.set l0, v0
 7 │     v2: ref<int32, borrowed, mutable, frame> = local.address l0
   │     ----------------------------------------------------------- borrow starts here
 8 │     local.set l0, v1
   │     ^^^^^^^^^^^^^^^^
 9 │     v3: int32 = load v2
10 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

#[test]
fn test_reject_local_set_while_borrow_is_stored_in_aggregate() {
    let mut program = TestProgram::mir(
        r#"
type Holder {
    value: ref<int32, borrowed, readonly>;
}

function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    v2: ref<int32, borrowed, readonly, frame> = local.address l0
    v3: Holder = aggregate (v2)
    local.set l0, v1
    v4: ref<int32, borrowed, readonly> = field.get v3, 0
    v5: int32 = load v4
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:13:5
   │
 9 │ entry(v0: int32, v1: int32):
10 │     local.set l0, v0
11 │     v2: ref<int32, borrowed, readonly, frame> = local.address l0
   │     ------------------------------------------------------------ borrow starts here
12 │     v3: Holder = aggregate (v2)
13 │     local.set l0, v1
   │     ^^^^^^^^^^^^^^^^
14 │     v4: ref<int32, borrowed, readonly> = field.get v3, 0
15 │     v5: int32 = load v4
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

#[test]
fn test_allow_change_after_aggregate_borrow_is_replaced() {
    let mut program = TestProgram::mir(
        r#"
type Holder {
    value: ref<int32, borrowed, readonly>;
}

function test(v0: int32, v1: int32, v2: int32): void {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32, v2: int32):
    local.set l0, v0
    local.set l1, v1
    v3: ref<int32, borrowed, readonly, frame> = local.address l0
    v4: Holder = aggregate (v3)
    v5: ref<int32, borrowed, readonly, frame> = local.address l1
    v6: Holder = field.set v4, 0, v5
    local.set l0, v2
    v7: ref<int32, borrowed, readonly> = field.get v6, 0
    v8: int32 = load v7
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_store_through_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
entry(v0: ref<int32, borrowed, readonly>, v1: int32):
    store v0, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[write-through-readonly-reference]: invalid MIR: cannot write through readonly reference
 ──▶ <test.dsm>:4:5
  │
2 │ function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
3 │ entry(v0: ref<int32, borrowed, readonly>, v1: int32):
4 │     store v0, v1
  │     ^^^^^^^^^^^^
5 │     return
6 │ }
  │

for more information about an error, run `destack explain write-through-readonly-reference`
"#,
    );
}

#[test]
fn test_reject_store_while_exclusive_borrow_is_live() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>, v1: int32): void {
entry(v0: ref<Box, borrowed, mutable>, v1: int32):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: Box = aggregate (v1)
    store v0, v3
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:11:5
   │
 7 │ function test(v0: ref<Box, borrowed, mutable>, v1: int32): void {
 8 │ entry(v0: ref<Box, borrowed, mutable>, v1: int32):
 9 │     v2: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     --------------------------------------------------------- borrow starts here
10 │     v3: Box = aggregate (v1)
11 │     store v0, v3
   │     ^^^^^^^^^^^^
12 │     v4: int32 = load v2
13 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

#[test]
fn test_allow_store_through_own_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>, v1: int32): void {
entry(v0: ref<Box, borrowed, mutable>, v1: int32):
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_load_through_alias_during_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, mutable>): int32 {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: Box = load v0
    v3: int32 = load v1
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-of-exclusively-borrowed-place]: cannot use exclusively borrowed place
  ──▶ <test.dsm>:10:5
   │
 7 │ function test(v0: ref<Box, borrowed, mutable>): int32 {
 8 │ entry(v0: ref<Box, borrowed, mutable>):
 9 │     v1: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     --------------------------------------------------------- borrow starts here
10 │     v2: Box = load v0
   │     ^^^^^^^^^^^^^^^^^
11 │     v3: int32 = load v1
12 │     return v3
   │

for more information about an error, run `destack explain use-of-exclusively-borrowed-place`
"#,
    );
}

#[test]
fn test_allow_store_through_live_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, mutable>, v1: int32): void {
entry(v0: ref<int32, borrowed, mutable>, v1: int32):
    store v0, v1
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_writable_borrow_through_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, borrowed, readonly>): void {
entry(v0: ref<Box, borrowed, readonly>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-through-readonly-reference]: cannot create a writable borrow through a readonly reference
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, borrowed, readonly>): void {
 7 │ entry(v0: ref<Box, borrowed, readonly>):
 8 │     v1: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-through-readonly-reference`
"#,
    );
}

#[test]
fn test_reject_local_read_during_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, exclusive, frame> = local.address l0
    v2: int32 = local.get l0
    v3: int32 = load v1
    v4: int32 = add v2, v3
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-of-exclusively-borrowed-place]: cannot use exclusively borrowed place
 ──▶ <test.dsm>:7:5
  │
4 │ entry(v0: int32):
5 │     local.set l0, v0
6 │     v1: ref<int32, borrowed, exclusive, frame> = local.address l0
  │     ------------------------------------------------------------- borrow starts here
7 │     v2: int32 = local.get l0
  │     ^^^^^^^^^^^^^^^^^^^^^^^^
8 │     v3: int32 = load v1
9 │     v4: int32 = add v2, v3
  │

for more information about an error, run `destack explain use-of-exclusively-borrowed-place`
"#,
    );
}

#[test]
fn test_reject_atomic_store_through_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<uint32, borrowed, readonly>, v1: uint32): void {
entry(v0: ref<uint32, borrowed, readonly>, v1: uint32):
    atomic.store v0, v1, release, scope(device)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[write-through-readonly-reference]: invalid MIR: cannot write through readonly reference
 ──▶ <test.dsm>:4:5
  │
2 │ function test(v0: ref<uint32, borrowed, readonly>, v1: uint32): void {
3 │ entry(v0: ref<uint32, borrowed, readonly>, v1: uint32):
4 │     atomic.store v0, v1, release, scope(device)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     return
6 │ }
  │

for more information about an error, run `destack explain write-through-readonly-reference`
"#,
    );
}

#[test]
fn test_reject_tensor_read_during_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: tensor<int32, managed, mutable, ()>): int32 {
entry(v0: tensor<int32, managed, mutable, ()>):
    v1: tensorView<int32, borrowed, exclusive, ()> = tensor.view v0, offsets(), sizes(), strides()
    v2: int32 = tensor.extract v0, []
    v3: int32 = tensor.load v1, []
    v4: int32 = add v2, v3
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-of-exclusively-borrowed-place]: cannot use exclusively borrowed place
 ──▶ <test.dsm>:5:5
  │
2 │ function test(v0: tensor<int32, managed, mutable, ()>): int32 {
3 │ entry(v0: tensor<int32, managed, mutable, ()>):
4 │     v1: tensorView<int32, borrowed, exclusive, ()> = tensor.view v0, offsets(), sizes(), strides()
  │     ---------------------------------------------------------------------------------------------- borrow starts here
5 │     v2: int32 = tensor.extract v0, []
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     v3: int32 = tensor.load v1, []
7 │     v4: int32 = add v2, v3
  │

for more information about an error, run `destack explain use-of-exclusively-borrowed-place`
"#,
    );
}
