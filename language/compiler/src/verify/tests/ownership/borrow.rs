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

    program.assert_error_borrow_conflict();
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

    program.assert_no_ownership_errors();
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

    program.assert_no_ownership_errors();
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

    program.assert_error_borrow_conflict();
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

    program.assert_no_ownership_errors();
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

    program.assert_error_borrow_conflict();
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

    program.assert_no_ownership_errors();
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

    program.assert_error_borrow_conflict();
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

    program.assert_error_borrow_conflict();
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

    program.assert_no_ownership_errors();
}

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

    program.assert_error_invalidation_of_borrowed_place();
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

    program.assert_error_write_through_readonly_reference();
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

    program.assert_error_invalidation_of_borrowed_place();
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

    program.assert_no_ownership_errors();
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

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_shared_managed_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, shared>): int32 {
entry(v0: ref<User, managed, mutable, shared>):
    v1: ref<int32, borrowed, readonly, shared> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_shared_managed_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, shared>): int32 {
entry(v0: ref<User, managed, mutable, shared>):
    v1: ref<int32, borrowed, exclusive, shared> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_error_exclusive_borrow_from_shared_managed();
}
