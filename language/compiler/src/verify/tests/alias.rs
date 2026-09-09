use crate::tests::{TestProgram, TestSession};

#[test]
fn test_allow_aliasable_mutable_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_mutable_borrows_from_distinct_parameters() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a, 'b>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<Box, borrowed, 'b, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: ref<Box, borrowed, 'b, mutable, local>):
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v3: ref<int32, borrowed, 'b, mutable, local> = field.address v1, 0
    v4: int32 = load v2
    v5: int32 = load v3
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_mutable_disjoint_fields() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Pair, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 1
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_mutable_dynamic_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4], v1: usize, v2: usize): void {
entry(v0: [int32; 4], v1: usize, v2: usize):
    v3: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v1
    v4: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v2
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
4 │     v3: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v1
  │     ------------------------------------------------------------------------- borrow starts here
5 │     v4: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v2
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     v5: int32 = load v3
7 │     v6: int32 = load v4
  │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_allow_mutable_constant_element_disjoint() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4]): void {
entry(v0: [int32; 4]):
    v1: uint64 = 0
    v2: uint64 = 1
    v3: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v1
    v4: ref<int32, borrowed, 'frame, mutable, local> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}
"#,
    );

    program.assert_verified();
}

/// Elements of a borrowed slice may be borrowed mutably twice, the slice aliasing its storage.
#[test]
fn test_allow_overlapping_element_borrows_of_a_borrowed_slice() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, borrowed, 'a, mutable, local>, v1: usize, v2: usize): void {
entry(v0: slice<int32, borrowed, 'a, mutable, local>, v1: usize, v2: usize):
    v3: ref<int32, borrowed, 'a, mutable, local> = element.address v0, v1
    v4: ref<int32, borrowed, 'a, mutable, local> = element.address v0, v2
    v5: int32 = load v3
    v6: int32 = load v4
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_mutable_disjoint_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, borrowed, 'a, mutable, local>): void {
entry(v0: slice<int32, borrowed, 'a, mutable, local>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: slice<int32, borrowed, 'a, mutable, local> = slice.view v0, v1, v2
    v4: slice<int32, borrowed, 'a, mutable, local> = slice.view v0, v2, v2
    v5: ref<int32, borrowed, 'a, mutable, local> = element.address v3, v1
    v6: ref<int32, borrowed, 'a, mutable, local> = element.address v4, v1
    v7: int32 = load v5
    v8: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}

/// Views of a borrowed slice may overlap, the slice aliasing its storage.
#[test]
fn test_allow_overlapping_views_of_a_borrowed_slice() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, borrowed, 'a, mutable, local>): void {
entry(v0: slice<int32, borrowed, 'a, mutable, local>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: uint64 = 1
    v4: slice<int32, borrowed, 'a, mutable, local> = slice.view v0, v1, v2
    v5: slice<int32, borrowed, 'a, mutable, local> = slice.view v0, v3, v2
    v6: ref<int32, borrowed, 'a, mutable, local> = element.address v4, v1
    v7: ref<int32, borrowed, 'a, mutable, local> = element.address v5, v1
    v8: int32 = load v6
    v9: int32 = load v7
    return
}
"#,
    );

    program.assert_verified();
}

/// Reborrows through a parameter alias freely, the storage behind it being unknown.
#[test]
fn test_allow_a_mutable_reborrow_beside_a_readonly_one_through_a_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, readonly, local> = field.address v0, 0
    v2: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_mutable_after_last_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
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

function test<'L>(v0: ref<Box, borrowed, 'L, mutable, local>, v1: boolean, v2: int32): ref<int32, borrowed, 'L, readonly, local> {
entry(v0: ref<Box, borrowed, 'L, mutable, local>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, 'L, readonly, local> = field.address v0, 0
    branch v1 => found(v3) | missing

found(v4: ref<int32, borrowed, 'L, readonly, local>):
    return v4

missing:
    v5: ref<int32, borrowed, 'L, mutable, local> = field.address v0, 0
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

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable, local>, v1: boolean, v2: int32): void {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<Pair, borrowed, 'a, mutable, local>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, 'a, readonly, local> = field.address v0, 0
    local.set l0, v3
    jump next

next:
    branch v1 => replace | done

replace:
    v4: ref<int32, borrowed, 'a, readonly, local> = field.address v0, 1
    local.set l0, v4
    v5: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    store v5, v2
    jump next

done:
    v6: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v7: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}

/// A readonly borrow kept live through one branch rejects an exclusive borrow after the join.
#[test]
fn test_reject_an_mutable_borrow_while_a_conditional_readonly_borrow_lives() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

function cond(): boolean {
    return false;
}

function borrowExclusive(cell: &Cell): void {}

function useRef(cell: &readonly Cell): void {}

export function preFreezeCond(): void {
    let u = Cell { value: 0 };
    let v = Cell { value: 3 };
    let w = &readonly u;
    if (cond()) {
        w = &readonly v;
    }
    borrowExclusive(&v);
    useRef(w);
}

export function preFreezeElse(): void {
    let u = Cell { value: 0 };
    let v = Cell { value: 3 };
    let w = &readonly u;
    if (cond()) {
        w = &readonly v;
    } else {
        borrowExclusive(&v);
    }
    useRef(w);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=21 column=21 span="&v" line_source="borrowExclusive(&v);"
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

export function getOrInsert(slot: &Slot, fallback: ^string): &readonly string {
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

function borrowExclusive(cell: &Cell): void {}

export function loopOverarchingAliasMut(): void {
    let v = Cell { value: 3 };
    const x = &v;
    x.value += 1;
    loop {
        borrow(&readonly v);
    }
}

export function blockOverarchingAliasMut(): void {
    let v = Cell { value: 3 };
    const x = &v;
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
        borrowExclusive(&v);
        x = &readonly v;
    }
}

export function whileAliasedMutCond(): void {
    let v = Cell { value: 3 };
    let w = Cell { value: 4 };
    let x = &readonly w;
    while (cond()) {
        borrowExclusive(&v);
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
/// @diagnostic.related line=25 column=15 span="&v" line_source="const x = &v;" message="borrow starts here"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=47 column=25 span="&v" line_source="borrowExclusive(&v);"
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

    next(&this): &Thing {
        return this;
    }
}

export function main(thing: &Thing): void {
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

function capitalize(value: &int32): void {}

export function nllFail(): void {
    let data = Data { a: 1, b: 2 };
    const c = &data.a;
    capitalize(c);
    data.a = 5;
    data.a = 6;
    data.a = 7;
    capitalize(c);
}

export function nllOk(): void {
    let data = Data { a: 1, b: 2 };
    const c = &data.a;
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
/// @diagnostic.related line=11 column=15 span="&data.a" line_source="const c = &data.a;" message="borrow starts here"
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

    set(&this, value: string): void {
        this.value = value;
    }
}

export function ok(map: &Map): &readonly string {
    loop {
        const found = map.get();
        if (map.flag) {
            return found;
        }
        map.set("next");
    }
}

"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// A call result reborrowing a parameter keeps that borrow live, a later write through the parameter aliasing it.
#[test]
fn test_allow_a_write_through_a_parameter_while_its_call_result_borrow_lives() {
    let session = TestSession::single(
        r#"
struct Map {
    flag: boolean;
    value: string;

    get(&readonly this): &readonly string {
        return &readonly this.value;
    }

    set(&this, value: string): void {
        this.value = value;
    }
}

export function refresh(map: &Map): &readonly string {
    const found = map.get();
    map.set("next");
    return found;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Distinct struct elements are disjoint at the stride their layout names.
#[test]
fn test_allow_mutable_disjoint_struct_elements() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test<'a>(v0: slice<Pair, borrowed, 'a, mutable, local>): void {
entry(v0: slice<Pair, borrowed, 'a, mutable, local>):
    v1: usize = 0
    v2: usize = 1
    v3: ref<Pair, borrowed, 'a, mutable, local> = element.address v0, v1
    v4: ref<Pair, borrowed, 'a, mutable, local> = element.address v0, v2
    v5: ref<int32, borrowed, 'a, mutable, local> = field.address v3, 0
    v6: ref<int32, borrowed, 'a, mutable, local> = field.address v4, 0
    v7: int32 = load v5
    v8: int32 = load v6
    return
}
"#,
    );

    program.assert_verified();
}

/// Overwriting an owned field through one handle while a borrow reaches through another verifies:
/// distinct handles are disjoint places and the old value stays until the park.
#[test]
fn test_allow_overwriting_an_owned_field_through_a_handle_under_a_borrow() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    item: ^Payload;

    constructor(item: ^Payload) {
        this.item = item;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

export function overwrite(a: Holder, b: Holder): int32 {
    const held = &readonly a.item;
    b.item = Payload { value: 1 };
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Overwriting an inline Drop struct field through a handle under a borrow verifies.
#[test]
fn test_allow_overwriting_an_inline_drop_field_through_a_handle_under_a_borrow() {
    let session = TestSession::single(
        r#"
struct Guard implements Drop {
    value: int32;

    drop(&this): void {}
}

class Holder {
    guard: Guard;

    constructor(guard: Guard) {
        this.guard = guard;
    }
}

function read(guard: &readonly Guard): int32 {
    return guard.value;
}

export function overwrite(a: Holder, b: Holder): int32 {
    const held = &readonly a.guard;
    b.guard = Guard { value: 1 };
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Popping through one handle while a borrow reaches into an element through another verifies.
#[test]
fn test_allow_popping_through_a_handle_under_an_element_borrow() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    items: Array<Payload>;

    constructor(items: Array<Payload>) {
        this.items = items;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

export function pop(a: Holder, b: Holder): int32 {
    const held = &readonly a.items[0];
    b.items.pop();
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Changing the case of a union field through a handle under a payload borrow verifies.
#[test]
fn test_allow_changing_a_union_case_through_a_handle_under_a_payload_borrow() {
    let session = TestSession::single(
        r#"
class Holder {
    value: string | number;

    constructor(value: string | number) {
        this.value = value;
    }
}

function read(text: &readonly string): isize {
    return text.length;
}

export function change(a: Holder, b: Holder): isize {
    if (a.value is string) {
        const held = &readonly a.value;
        b.value = 3;
        return read(held);
    }
    return 0;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Clearing an optional owned field through a handle under a payload borrow verifies.
#[test]
fn test_allow_clearing_an_optional_owned_field_through_a_handle_under_a_borrow() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

export function clear(a: Holder, b: Holder): int32 {
    if (a.slot !== undefined) {
        const held = &readonly a.slot;
        b.slot = undefined;
        return read(held);
    }
    return 0;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// A borrow into an owned field survives a park and a replacement through another handle.
#[test]
fn test_allow_a_borrow_into_a_replaced_field_across_a_park() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    item: ^Payload;

    constructor(item: ^Payload) {
        this.item = item;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

async function pause(): Promise<void> {}

export async function replace(a: Holder, b: Holder): Promise<int32> {
    const held = &readonly a.item;
    await pause();
    b.item = Payload { value: 1 };
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Mutating through a second handle inside a method holding a borrow of its own storage verifies.
#[test]
fn test_allow_reentrant_mutation_through_a_second_handle_inside_a_method() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    item: ^Payload;

    constructor(item: ^Payload) {
        this.item = item;
    }

    poke(&this, other: Holder): int32 {
        const held = &readonly this.item;
        other.item = Payload { value: 1 };
        return held.value;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Iterating one handle's items while pushing through another handle verifies.
#[test]
fn test_allow_pushing_through_a_handle_while_iterating_its_items() {
    let session = TestSession::single(
        r#"
class Holder {
    items: Array<int32>;

    constructor(items: Array<int32>) {
        this.items = items;
    }
}

export function grow(a: Holder, b: Holder): int32 {
    let total = 0;
    for (const item of a.items) {
        b.items.push(1);
        total += item;
    }
    return total;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Clearing a boxed non-Copy payload through a handle under a borrow into the box verifies.
#[test]
fn test_allow_clearing_a_boxed_payload_through_a_handle_under_a_borrow() {
    let session = TestSession::single(
        r#"
struct Payload {
    steps: ^Array<int32>;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

function read(payload: &readonly Payload): isize {
    return payload.steps.length;
}

export function clear(a: Holder, b: Holder): isize {
    if (a.slot !== undefined) {
        const held = &readonly a.slot;
        b.slot = undefined;
        return read(held);
    }
    return 0;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Pushing onto an owned array while a borrow iterates its items is rejected by the borrow check.
#[test]
fn test_reject_pushing_onto_an_owned_array_while_iterating_its_items() {
    let session = TestSession::single(
        r#"
export function grow(): int32 {
    let items: ^int64[] = Array.of(1, 2, 3);
    let total = 0;
    for (const item of &readonly items) {
        items.push(1);
        total += 1;
    }
    return total;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=6 column=9 span="items.push(1)" line_source="items.push(1);"
/// @diagnostic.related line=5 column=24 span="&readonly items" line_source="for (const item of &readonly items) {" message="borrow starts here"
"#,
    );
}

/// Iterating a readonly borrow of an owned array verifies while nothing writes the array.
#[test]
fn test_allow_iterating_a_readonly_borrow_of_an_owned_array() {
    let session = TestSession::single(
        r#"
export function total(items: ^int64[]): int64 {
    let total = 0;
    for (const item of &readonly items) {
        total += 1;
    }
    return total;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}
