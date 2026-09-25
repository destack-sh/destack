use crate::tests::TestProgram;

/// A readonly borrow of shared managed storage verifies.
#[test]
fn test_allow_shared_managed_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, shared>): int32 {
entry(v0: ref<User, managed, mutable, shared>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}

/// An exclusive borrow of shared managed storage reports a diagnostic.
#[test]
fn test_reject_shared_managed_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, shared>): int32 {
entry(v0: ref<User, managed, mutable, shared>):
    v1: ref<int32, borrowed, 'managed, mutable> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[mutable-borrow-from-shared-storage]: cannot borrow shared storage mutably
  ──▶ <test.tsppm>:8:5
   │
 6 │ function test(v0: ref<User, managed, mutable, shared>): int32 {
 7 │ entry(v0: ref<User, managed, mutable, shared>):
 8 │     v1: ref<int32, borrowed, 'managed, mutable> = address (*v0).0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     v2: int32 = load (*v1)
10 │     return v2
   │

for more information about an error, run `tspp explain mutable-borrow-from-shared-storage`
"#,
    );
}

/// An exclusive borrow of a shared global reports a diagnostic.
#[test]
fn test_reject_shared_global_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
shared global value: int32 = 0

function test(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, mutable> = address @value
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[mutable-borrow-from-shared-storage]: cannot borrow shared storage mutably
 ──▶ <test.tsppm>:6:5
  │
4 │ function test(): int32 {
5 │ entry:
6 │     v0: ref<int32, borrowed, 'static, mutable> = address @value
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     v1: int32 = load (*v0)
8 │     return v1
  │

for more information about an error, run `tspp explain mutable-borrow-from-shared-storage`
"#,
    );
}

/// Passing shared storage to an exclusive parameter reports a diagnostic.
#[test]
fn test_reject_shared_storage_passed_mutably() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

external function update<'a>(ref<User, borrowed, 'a, mutable>): void

function test(v0: ref<User, managed, mutable, shared>): void {
entry(v0: ref<User, managed, mutable, shared>):
    v1: ref<User, borrowed, 'managed, mutable> = address (*v0)
    call update(v1): <'a>(ref<User, borrowed, 'a, mutable>) => void
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[mutable-borrow-from-shared-storage]: cannot borrow shared storage mutably
  ──▶ <test.tsppm>:10:5
   │
 8 │ function test(v0: ref<User, managed, mutable, shared>): void {
 9 │ entry(v0: ref<User, managed, mutable, shared>):
10 │     v1: ref<User, borrowed, 'managed, mutable> = address (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     call update(v1): <'a>(ref<User, borrowed, 'a, mutable>) => void
12 │     return
   │

for more information about an error, run `tspp explain mutable-borrow-from-shared-storage`
"#,
    );
}

/// Storing a frame borrow into managed storage reports a diagnostic.
#[test]
fn test_reject_frame_borrow_stored_in_managed_storage() {
    let mut program = TestProgram::mir(
        r#"
type Owner {
    value: int32;
}

type View<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test<'a>(v0: ref<Owner, unique, mutable>, v1: ref<View<'a>, managed, mutable, local>): void {
entry(v0: ref<Owner, unique, mutable>, v1: ref<View<'a>, managed, mutable, local>):
    v2: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v3: ref<ref<int32, borrowed, 'a, readonly>, borrowed, 'managed, mutable> = address (*v1).0
    store (*v3), v2
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:14:5
   │
12 │     v2: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
13 │     v3: ref<ref<int32, borrowed, 'a, readonly>, borrowed, 'managed, mutable> = address (*v1).0
14 │     store (*v3), v2
   │     ^^^^^^^^^^^^^^^
15 │     return
16 │ }
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// Exclusive borrows of a heap field and a global verify together.
#[test]
fn test_allow_disjoint_heap_and_global_borrows() {
    let mut program = TestProgram::mir(
        r#"
global value: int32 = 0

type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): int32 {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v2: ref<int32, borrowed, 'static, mutable> = address @value
    v3: int32 = load (*v1)
    v4: int32 = load (*v2)
    v5: int32 = add v3, v4
    return v5
}
"#,
    );

    program.assert_verified();
}

/// Exclusive borrows of fields in distinct allocations verify together.
#[test]
fn test_allow_mutable_borrows_from_distinct_allocations() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): int32 {
entry:
    v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v1: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v2: ref<int32, borrowed, 'managed, mutable> = address (*v0).0
    v3: ref<int32, borrowed, 'managed, mutable> = address (*v1).0
    v4: int32 = load (*v2)
    v5: int32 = load (*v3)
    v6: int32 = add v4, v5
    return v6
}
"#,
    );

    program.assert_verified();
}

/// Reject an allocation outside its handle heap as invalid MIR.
#[test]
fn test_reject_an_allocation_outside_its_handle_heap() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(): void {
entry:
    v0: ref<User, managed, mutable, shared> = new.zeroed User, local
    return
}
"#,
    );

    program.assert_invalid_mir(
        r#"
invalid MIR: an allocation of a shared heap handle in the local heap in 'test'
"#,
    );
}
