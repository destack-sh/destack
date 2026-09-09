use crate::tests::{TestProgram, TestSession};

#[test]
fn test_reject_local_set_while_borrowed() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    v2: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
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
 7 │     v2: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
   │     ------------------------------------------------------------------- borrow starts here
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
type Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    v2: ref<int32, borrowed, 'frame, readonly, frame> = local.address l0
    v3: Holder<'frame & local> = aggregate (v2)
    local.set l0, v1
    v4: ref<int32, borrowed, 'frame, readonly, local> = field.get v3, 0
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
11 │     v2: ref<int32, borrowed, 'frame, readonly, frame> = local.address l0
   │     -------------------------------------------------------------------- borrow starts here
12 │     v3: Holder<'frame & local> = aggregate (v2)
13 │     local.set l0, v1
   │     ^^^^^^^^^^^^^^^^
14 │     v4: ref<int32, borrowed, 'frame, readonly, local> = field.get v3, 0
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
type Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test(v0: int32, v1: int32, v2: int32): void {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32, v2: int32):
    local.set l0, v0
    local.set l1, v1
    v3: ref<int32, borrowed, 'frame, readonly, frame> = local.address l0
    v4: Holder<'frame & local> = aggregate (v3)
    v5: ref<int32, borrowed, 'frame, readonly, frame> = local.address l1
    v6: Holder<'frame & local> = field.set v4, 0, v5
    local.set l0, v2
    v7: ref<int32, borrowed, 'frame, readonly, local> = field.get v6, 0
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
function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: int32): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: int32):
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
2 │ function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: int32): void {
3 │ entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: int32):
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
fn test_reject_store_while_mutable_borrow_is_live() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32):
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
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
 7 │ function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32): void {
 8 │ entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32):
 9 │     v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
   │     ------------------------------------------------------------------ borrow starts here
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
fn test_allow_store_through_own_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: int32):
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

/// A parameter may be read while a field reborrowed through it is live, the storage aliasing.
#[test]
fn test_allow_a_load_through_a_parameter_during_its_reborrow() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v2: Box = load v0
    v3: int32 = load v1
    return v3
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_store_through_live_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32): void {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32):
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

function test<'a>(v0: ref<Box, borrowed, 'a, readonly, local>): void {
entry(v0: ref<Box, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-through-readonly-reference]: cannot create a writable borrow through a readonly reference
  ──▶ <test.dsm>:8:5
   │
 6 │ function test<'a>(v0: ref<Box, borrowed, 'a, readonly, local>): void {
 7 │ entry(v0: ref<Box, borrowed, 'a, readonly, local>):
 8 │     v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-through-readonly-reference`
"#,
    );
}

#[test]
fn test_reject_local_read_during_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = local.get l0
    v3: int32 = load v1
    v4: int32 = add v2, v3
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-of-mutably-borrowed-place]: cannot use mutably borrowed place
 ──▶ <test.dsm>:7:5
  │
4 │ entry(v0: int32):
5 │     local.set l0, v0
6 │     v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
  │     ------------------------------------------------------------------- borrow starts here
7 │     v2: int32 = local.get l0
  │     ^^^^^^^^^^^^^^^^^^^^^^^^
8 │     v3: int32 = load v1
9 │     v4: int32 = add v2, v3
  │

for more information about an error, run `destack explain use-of-mutably-borrowed-place`
"#,
    );
}

#[test]
fn test_reject_atomic_store_through_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<uint32, borrowed, 'a, readonly, local>, v1: uint32): void {
entry(v0: ref<uint32, borrowed, 'a, readonly, local>, v1: uint32):
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
2 │ function test<'a>(v0: ref<uint32, borrowed, 'a, readonly, local>, v1: uint32): void {
3 │ entry(v0: ref<uint32, borrowed, 'a, readonly, local>, v1: uint32):
4 │     atomic.store v0, v1, release, scope(device)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     return
6 │ }
  │

for more information about an error, run `destack explain write-through-readonly-reference`
"#,
    );
}

/// Nested fields of an owned value take assignments.
#[test]
fn test_allow_assignments_to_nested_fields() {
    let session = TestSession::single(
        r#"
struct B {
    a: int32;
}

struct A {
    a: int32;
    w: B;
}

export function main(): void {
    let p = A { a: 1, w: B { a: 1 } };
    p.a = 2;
    p.w.a = 2;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// A write inside a loop conflicts with a borrow the loop uses later.
#[test]
fn test_reject_a_write_before_a_later_use_of_the_borrow_inside_a_loop() {
    let session = TestSession::single(
        r#"
struct Record {
    field: int32;
}

function length(value: &readonly int32): int32 {
    return 0;
}

export function nllFail(): void {
    let record = Record { field: 1 };
    const value = &record.field;
    loop {
        record.field += 1;
        length(value);
        return;
    }
}

export function nllOk(): void {
    let record = Record { field: 1 };
    const value = &record.field;
    loop {
        record.field += 1;
        return;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=use-of-mutably-borrowed-place message="cannot use mutably borrowed place"
/// @diagnostic.label line=14 column=9 span="record.field += 1" line_source="record.field += 1;"
/// @diagnostic.related line=12 column=19 span="&record.field" line_source="const value = &record.field;" message="borrow starts here"
"#);
}

/// A reference local reassigned across a merge still names one place through its dereference.
#[test]
fn test_reject_a_write_through_a_reassigned_reference_while_its_borrow_lives() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

struct Pair {
    a: int32;
    b: Token;
}

function read(value: &readonly int32): void {}  function consume(pair: Pair): void {}  export function write(flag: boolean): void {
    let left = Pair { a: 1, b: Token { id: 2 } };
    let right = Pair { a: 3, b: Token { id: 4 } };
    let target = &left;
    if (flag) {
        target = &right;
    }
    const borrowed = &readonly target.a;
    target.a = 5;
    read(borrowed);
}

export function borrowFieldAfterWholeMove(): void {
    const pair = Pair { a: 1, b: Token { id: 2 } };
    consume(pair);
    read(&readonly pair.a);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=21 column=5 span="target.a = 5" line_source="target.a = 5;"
/// @diagnostic.related line=20 column=22 span="&readonly target.a" line_source="const borrowed = &readonly target.a;" message="borrow starts here"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=28 column=10 span="&readonly pair.a" line_source="read(&readonly pair.a);"
/// @diagnostic.related line=27 column=13 span="pair" line_source="consume(pair);" message="value moved here"
"#,
    );
}

/// Exclusive borrows into the payloads of two different cases are disjoint.
#[test]
fn test_allow_mutable_borrows_into_different_variant_cases() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

function test(v0: int32, v1: int64, v6: Either): void {
    local l0: Either

entry(v0: int32, v1: int64, v6: Either):
    local.set l0, v6
    v2: ref<Either, borrowed, 'frame, mutable, frame> = local.project l0
    v3: ref<int32, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 0
    v4: ref<int64, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 1
    store v3, v0
    store v4, v1
    v5: int32 = load v3
    return
}
"#,
    );

    program.assert_verified();
}

/// Two exclusive borrows into one case's payload conflict.
#[test]
fn test_reject_mutable_borrows_into_one_variant_case() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

function test(v0: int32, v1: int32, v6: Either): void {
    local l0: Either

entry(v0: int32, v1: int32, v6: Either):
    local.set l0, v6
    v2: ref<Either, borrowed, 'frame, mutable, frame> = local.project l0
    v3: ref<int32, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 0
    v4: ref<int32, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 0
    store v3, v0
    store v4, v1
    v5: int32 = load v3
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:11:5
   │
 8 │     local.set l0, v6
 9 │     v2: ref<Either, borrowed, 'frame, mutable, frame> = local.project l0
10 │     v3: ref<int32, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 0
   │     -------------------------------------------------------------------------------- borrow starts here
11 │     v4: ref<int32, borrowed, 'frame, mutable, frame> = variant.payload.address v2, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     store v3, v0
13 │     store v4, v1
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}
