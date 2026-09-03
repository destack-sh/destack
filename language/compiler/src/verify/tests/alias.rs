use crate::tests::{TestProgram, TestSession};

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

/// A readonly borrow kept live through one branch rejects an exclusive borrow after the join.
#[test]
fn test_reject_an_exclusive_borrow_while_a_conditional_readonly_borrow_lives() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

function cond(): boolean {
    return false;
}

function borrowExclusive(cell: &exclusive Cell): void {}

function useRef(cell: &readonly Cell): void {}

export function preFreezeCond(): void {
    let u = Cell { value: 0 };
    let v = Cell { value: 3 };
    let w = &readonly u;
    if (cond()) {
        w = &readonly v;
    }
    borrowExclusive(&exclusive v);
    useRef(w);
}

export function preFreezeElse(): void {
    let u = Cell { value: 0 };
    let v = Cell { value: 3 };
    let w = &readonly u;
    if (cond()) {
        w = &readonly v;
    } else {
        borrowExclusive(&exclusive v);
    }
    useRef(w);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=21 column=21 span="&exclusive v" line_source="borrowExclusive(&exclusive v);"
/// @diagnostic.related line=19 column=13 span="&readonly v" line_source="w = &readonly v;" message="borrow starts here"
"#);
}

/// A borrow returned from one branch leaves the other branch free to write.
#[test]
fn test_allow_a_write_after_a_borrow_returned_only_from_another_branch() {
    let session = TestSession::single(
        r#"
struct Slot {
    flag: boolean;
    value: ^string;
}

export function getOrInsert(slot: &exclusive Slot, fallback: ^string): &readonly string {
    const current = &readonly slot.value;
    if (slot.flag) {
        return current;
    }
    slot.value = fallback;
    return &readonly slot.value;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Borrows carried across loops conflict with the accesses each iteration makes.
#[test]
fn test_track_borrows_across_loops() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

function cond(): boolean {
    return false;
}

function borrow(cell: &readonly Cell): void {}

function borrowExclusive(cell: &exclusive Cell): void {}

export function loopOverarchingAliasMut(): void {
    let v = Cell { value: 3 };
    const x = &exclusive v;
    x.value += 1;
    loop {
        borrow(&readonly v);
    }
}

export function blockOverarchingAliasMut(): void {
    let v = Cell { value: 3 };
    const x = &exclusive v;
    for (let i = 0; i < 3; i++) {
        borrow(&readonly v);
    }
    x.value = 5;
}

export function whileAliasedMut(): void {
    let v = Cell { value: 3 };
    let w = Cell { value: 4 };
    let x = &readonly w;
    while (cond()) {
        borrowExclusive(&exclusive v);
        x = &readonly v;
    }
}

export function whileAliasedMutCond(): void {
    let v = Cell { value: 3 };
    let w = Cell { value: 4 };
    let x = &readonly w;
    while (cond()) {
        borrowExclusive(&exclusive v);
        if (cond()) {
            x = &readonly v;
        }
    }
    borrow(x);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=27 column=16 span="&readonly v" line_source="borrow(&readonly v);"
/// @diagnostic.related line=25 column=15 span="&exclusive v" line_source="const x = &exclusive v;" message="borrow starts here"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=47 column=25 span="&exclusive v" line_source="borrowExclusive(&exclusive v);"
/// @diagnostic.related line=49 column=17 span="&readonly v" line_source="x = &readonly v;" message="borrow starts here"
"#);
}

/// Reassigning the reference binding kills the loan it held.
#[test]
fn test_kill_a_loan_when_its_reference_binding_is_reassigned() {
    let session = TestSession::single(
        r#"
struct Thing {
    value: int32;

    next(&exclusive this): &exclusive Thing {
        return this;
    }
}

export function main(thing: &exclusive Thing): void {
    let temp = thing;
    loop {
        const v = temp.next();
        temp = v;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// A loan ends at the last use of its reference.
#[test]
fn test_end_a_loan_at_the_last_use_of_its_reference() {
    let session = TestSession::single(
        r#"
struct Data {
    a: int32;
    b: int32;
}

function capitalize(value: &exclusive int32): void {}

export function nllFail(): void {
    let data = Data { a: 1, b: 2 };
    const c = &exclusive data.a;
    capitalize(c);
    data.a = 5;
    data.a = 6;
    data.a = 7;
    capitalize(c);
}

export function nllOk(): void {
    let data = Data { a: 1, b: 2 };
    const c = &exclusive data.a;
    capitalize(c);
    data.a = 5;
    data.a = 6;
    data.a = 7;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=13 column=5 span="data.a = 5" line_source="data.a = 5;"
/// @diagnostic.related line=11 column=15 span="&exclusive data.a" line_source="const c = &exclusive data.a;" message="borrow starts here"
"#);
}

/// A borrow returned from one arm leaves the other arm free to write, and a write before the return conflicts.
#[test]
fn test_allow_a_write_in_the_arm_that_does_not_return_the_borrow() {
    let session = TestSession::single(
        r#"
struct Map {
    flag: boolean;
    value: string;

    get(&readonly this): &readonly string {
        return &readonly this.value;
    }

    set(&exclusive this, value: string): void {
        this.value = value;
    }
}

export function ok(map: &exclusive Map): &readonly string {
    loop {
        const found = map.get();
        if (map.flag) {
            return found;
        }
        map.set("next");
    }
}

export function err(map: &exclusive Map): &readonly string {
    loop {
        const found = map.get();
        map.set("next");
        return found;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=28 column=9 span="map.set(\"next\")" line_source="map.set(\"next\");"
/// @diagnostic.related line=27 column=23 span="map.get()" line_source="const found = map.get();" message="borrow starts here"
"#);
}

/// A call result reborrowing its argument keeps that borrow live, so a later exclusive reborrow conflicts.
#[test]
fn test_reject_an_exclusive_reborrow_while_a_call_result_reborrow_lives() {
    let session = TestSession::single(
        r#"
struct Map {
    flag: boolean;
    value: string;

    get(&readonly this): &readonly string {
        return &readonly this.value;
    }

    set(&exclusive this, value: string): void {
        this.value = value;
    }
}

export function refresh(map: &exclusive Map): &readonly string {
    const found = map.get();
    map.set("next");
    return found;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=17 column=5 span="map.set(\"next\")" line_source="map.set(\"next\");"
/// @diagnostic.related line=16 column=19 span="map.get()" line_source="const found = map.get();" message="borrow starts here"
"#);
}

/// Distinct struct elements are disjoint at the stride their layout names.
#[test]
fn test_allow_exclusive_disjoint_struct_elements() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test(v0: slice<Pair, borrowed, mutable>): void {
entry(v0: slice<Pair, borrowed, mutable>):
    v1: usize = 0
    v2: usize = 1
    v3: ref<Pair, borrowed, exclusive> = element.address v0, v1
    v4: ref<Pair, borrowed, exclusive> = element.address v0, v2
    v5: ref<int32, borrowed, exclusive> = field.address v3, 0
    v6: ref<int32, borrowed, exclusive> = field.address v4, 0
    v7: int32 = load v5
    v8: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}
