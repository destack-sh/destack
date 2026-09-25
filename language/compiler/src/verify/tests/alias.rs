use crate::tests::{TestProgram, TestSession};

/// Two mutable borrows of one field alias without conflict.
#[test]
fn test_allow_aliasable_mutable_overlap() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v3: int32 = load (*v1)
    v4: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verified();
}

/// Mutable borrows through two distinct parameters coexist.
#[test]
fn test_allow_mutable_borrows_from_distinct_parameters() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a, 'b>(v0: ref<Box, borrowed, 'a, mutable>, v1: ref<Box, borrowed, 'b, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>, v1: ref<Box, borrowed, 'b, mutable>):
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v3: ref<int32, borrowed, 'b, mutable> = address (*v1).0
    v4: int32 = load (*v2)
    v5: int32 = load (*v3)
    return
}
"#,
    );

    program.assert_verified();
}

/// Mutable borrows of two different fields coexist.
#[test]
fn test_allow_mutable_disjoint_fields() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable>): void {
entry(v0: ref<Pair, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).1
    v3: int32 = load (*v1)
    v4: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verified();
}

/// Exclusive borrows at two dynamic indices may overlap and conflict.
#[test]
fn test_reject_mutable_dynamic_element_overlap() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4], v1: usize, v2: usize): void {
entry(v0: [int32; 4], v1: usize, v2: usize):
    v3: ref<int32, borrowed, 'frame, exclusive> = address (v0)[v1]
    v4: ref<int32, borrowed, 'frame, exclusive> = address (v0)[v2]
    v5: int32 = load (*v3)
    v6: int32 = load (*v4)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
 ──▶ <test.tsppm>:5:5
  │
2 │ function test(v0: [int32; 4], v1: usize, v2: usize): void {
3 │ entry(v0: [int32; 4], v1: usize, v2: usize):
4 │     v3: ref<int32, borrowed, 'frame, exclusive> = address (v0)[v1]
  │     -------------------------------------------------------------- borrow starts here
5 │     v4: ref<int32, borrowed, 'frame, exclusive> = address (v0)[v2]
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │     v5: int32 = load (*v3)
7 │     v6: int32 = load (*v4)
  │

for more information about an error, run `tspp explain borrow-conflict`
"#,
    );
}

/// Mutable borrows at two distinct constant indices coexist.
#[test]
fn test_allow_mutable_constant_element_disjoint() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: [int32; 4]): void {
entry(v0: [int32; 4]):
    v1: uint64 = 0
    v2: uint64 = 1
    v3: ref<int32, borrowed, 'frame, mutable> = address (v0)[v1]
    v4: ref<int32, borrowed, 'frame, mutable> = address (v0)[v2]
    v5: int32 = load (*v3)
    v6: int32 = load (*v4)
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
function test<'a>(v0: slice<int32, borrowed, 'a, mutable>, v1: usize, v2: usize): void {
entry(v0: slice<int32, borrowed, 'a, mutable>, v1: usize, v2: usize):
    v3: ref<int32, borrowed, 'a, mutable> = address (*v0)[v1]
    v4: ref<int32, borrowed, 'a, mutable> = address (*v0)[v2]
    v5: int32 = load (*v3)
    v6: int32 = load (*v4)
    return
}
"#,
    );

    program.assert_verified();
}

/// Mutable borrows into two disjoint constant slice ranges coexist.
#[test]
fn test_allow_mutable_disjoint_slice_ranges() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, borrowed, 'a, mutable>): void {
entry(v0: slice<int32, borrowed, 'a, mutable>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: slice<int32, borrowed, 'a, mutable> = address (*v0)[v1; v2]
    v4: slice<int32, borrowed, 'a, mutable> = address (*v0)[v2; v2]
    v5: ref<int32, borrowed, 'a, mutable> = address (*v3)[v1]
    v6: ref<int32, borrowed, 'a, mutable> = address (*v4)[v1]
    v7: int32 = load (*v5)
    v8: int32 = load (*v6)
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
function test<'a>(v0: slice<int32, borrowed, 'a, mutable>): void {
entry(v0: slice<int32, borrowed, 'a, mutable>):
    v1: uint64 = 0
    v2: uint64 = 2
    v3: uint64 = 1
    v4: slice<int32, borrowed, 'a, mutable> = address (*v0)[v1; v2]
    v5: slice<int32, borrowed, 'a, mutable> = address (*v0)[v3; v2]
    v6: ref<int32, borrowed, 'a, mutable> = address (*v4)[v1]
    v7: ref<int32, borrowed, 'a, mutable> = address (*v5)[v1]
    v8: int32 = load (*v6)
    v9: int32 = load (*v7)
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

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, readonly> = address (*v0).0
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v3: int32 = load (*v1)
    v4: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verified();
}

/// A mutable borrow may start after the last use of a readonly borrow.
#[test]
fn test_allow_mutable_after_last_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    v3: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v4: int32 = load (*v3)
    return
}
"#,
    );

    program.assert_verified();
}

/// A borrow returned on one branch leaves the other branch free to write.
#[test]
fn test_allow_change_after_borrow_returns_on_other_branch() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'L>(v0: ref<Box, borrowed, 'L, mutable>, v1: boolean, v2: int32): ref<int32, borrowed, 'L, readonly> {
entry(v0: ref<Box, borrowed, 'L, mutable>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, 'L, readonly> = address (*v0).0
    branch v1 => found(v3) | missing

found(v4: ref<int32, borrowed, 'L, readonly>):
    return v4

missing:
    v5: ref<int32, borrowed, 'L, mutable> = address (*v0).0
    store (*v5), v2
    return v5
}
"#,
    );

    program.assert_verified();
}

/// A write in a loop is allowed once the loop-carried borrow moves to another field.
#[test]
fn test_allow_change_after_loop_carried_borrow_is_replaced() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable>, v1: boolean, v2: int32): void {
    local l0: ref<int32, borrowed, 'a, readonly>

entry(v0: ref<Pair, borrowed, 'a, mutable>, v1: boolean, v2: int32):
    v3: ref<int32, borrowed, 'a, readonly> = address (*v0).0
    store l0, v3
    jump next

next:
    branch v1 => replace | done

replace:
    v4: ref<int32, borrowed, 'a, readonly> = address (*v0).1
    store l0, v4
    v5: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    store (*v5), v2
    jump next

done:
    v6: ref<int32, borrowed, 'a, readonly> = load l0
    v7: int32 = load (*v6)
    return
}
"#,
    );

    program.assert_verified();
}

/// Allow a writable argument over Copy storage while a conditional readonly loan lives.
#[test]
fn test_allow_a_copy_writable_argument_while_a_conditional_readonly_loan_lives() {
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"

"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Allow a writable argument over Copy storage under a readonly loan carried across loops.
#[test]
fn test_allow_a_copy_writable_argument_under_a_readonly_loan_carried_across_loops() {
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"

"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Allow Copy writes under an aliasable loan before its last use.
#[test]
fn test_allow_copy_writes_under_an_aliasable_loan_before_its_last_use() {
    let session = TestSession::single(
        r#"
struct Data {
    a: int32;
    b: int32;
}

function capitalize(value: &int32): void {}

export function useAfterWrites(): void {
    let data = Data { a: 1, b: 2 };
    const c = &data.a;
    capitalize(c);
    data.a = 5;
    data.a = 6;
    data.a = 7;
    capitalize(c);
}

export function writesAfterUse(): void {
    let data = Data { a: 1, b: 2 };
    const c = &data.a;
    capitalize(c);
    data.a = 5;
    data.a = 6;
    data.a = 7;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"

"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Allow replacing a handle field while a borrow of the object it held lives.
#[test]
fn test_allow_replacing_a_handle_field_while_a_borrow_of_its_object_lives() {
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"

"#,
    );
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

function test<'a>(v0: slice<Pair, borrowed, 'a, mutable>): void {
entry(v0: slice<Pair, borrowed, 'a, mutable>):
    v1: usize = 0
    v2: usize = 1
    v3: ref<Pair, borrowed, 'a, mutable> = address (*v0)[v1]
    v4: ref<Pair, borrowed, 'a, mutable> = address (*v0)[v2]
    v5: ref<int32, borrowed, 'a, mutable> = address (*v3).0
    v6: ref<int32, borrowed, 'a, mutable> = address (*v4).0
    v7: int32 = load (*v5)
    v8: int32 = load (*v6)
    return
}
"#,
    );

    program.assert_verified();
}

/// Overwriting an owned field through one handle under a borrow through another verifies.
///
/// A write below a handle releases nothing, since the heap retains the replaced value.
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Borrowing a non-Copy payload in place through a handle is rejected.
#[test]
fn test_reject_borrowing_a_non_copy_payload_through_a_handle() {
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=borrow-of-aliasable-variant message="cannot borrow an inline variant payload through aliasable access"
/// @diagnostic.label line=20 column=22 span="&readonly a.slot" line_source="const held = &readonly a.slot;"
"#,
    );
}

/// A field of a narrowed non-Copy payload reads as a direct load through a handle.
#[test]
fn test_allow_reading_a_narrowed_non_copy_payload_through_a_handle() {
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

export function read(a: Holder, b: Holder): int32 {
    if (a.slot !== undefined) {
        const value = a.slot.value;
        b.slot = undefined;
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// A non-Copy payload of a frame-owned object borrows in place.
#[test]
fn test_allow_borrowing_a_non_copy_payload_of_a_frame_owned_object() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(&exclusive this, slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

function read(payload: &readonly Payload): int32 {
    return payload.value;
}

export function inspect(slot: ^Payload | undefined): int32 {
    const holder: ^Holder = new Holder(slot);
    if (holder.slot !== undefined) {
        const held = &readonly holder.slot;
        return read(held);
    }
    return 0;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// A borrow into an owned field survives a replacement through another handle, then a park.
///
/// The heap retains the replaced value below the handle, so the borrow stays valid after the park.
#[test]
fn test_allow_a_borrow_into_a_replaced_field_used_after_a_park() {
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
    b.item = Payload { value: 1 };
    await pause();
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
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

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Overwriting an owned field of a frame-owned object under an exclusive borrow invalidates it.
#[test]
fn test_reject_overwriting_an_owned_field_of_a_frame_owned_object_under_an_exclusive_borrow() {
    let session = TestSession::single(
        r#"
import { Box } from "tspp:memory";

class Holder {
    item: ^Box<int32>;

    constructor(item: ^Box<int32>) {
        this.item = item;
    }
}

function read(item: &readonly Box<int32>): void {}

export function overwrite(a: ^Holder): void {
    const held = &exclusive a.item;
    a.item = Box.new(1);
    read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=16 column=5 span="a.item = Box.new(1)" line_source="a.item = Box.new(1);"
/// @diagnostic.related line=15 column=18 span="&exclusive a.item" line_source="const held = &exclusive a.item;" message="borrow starts here"
"#,
    );
}

/// Overwriting an owned field of a frame-owned object under a readonly borrow invalidates it.
#[test]
fn test_reject_overwriting_an_owned_field_of_a_frame_owned_object_under_a_readonly_borrow() {
    let session = TestSession::single(
        r#"
import { Box } from "tspp:memory";

class Holder {
    item: ^Box<int32>;

    constructor(item: ^Box<int32>) {
        this.item = item;
    }
}

function read(item: &readonly Box<int32>): void {}

export function overwrite(a: ^Holder): void {
    const held = &readonly a.item;
    a.item = Box.new(1);
    read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=16 column=5 span="a.item = Box.new(1)" line_source="a.item = Box.new(1);"
/// @diagnostic.related line=15 column=18 span="&readonly a.item" line_source="const held = &readonly a.item;" message="borrow starts here"
"#,
    );
}

/// Popping a frame-owned array under a borrow of one owned element invalidates the borrow.
#[test]
fn test_reject_popping_a_frame_owned_array_under_an_element_borrow() {
    let session = TestSession::single(
        r#"
import { Box } from "tspp:memory";

function read(item: &immutable Box<int32>): int32 {
    return 0;
}

export function pop(items: ^Array<^Box<int32>>): int32 {
    const held = &immutable items[0];
    items.pop();
    return read(held);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=10 column=5 span="items.pop()" line_source="items.pop();"
/// @diagnostic.related line=9 column=18 span="&immutable items[0]" line_source="const held = &immutable items[0];" message="borrow starts here"
"#,
    );
}

/// Pushing to a frame-owned array conflicts with the readonly borrow its iteration holds.
#[test]
fn test_reject_pushing_to_a_frame_owned_array_while_iterating_it() {
    let session = TestSession::single(
        r#"
export function grow(items: ^int64[]): int64 {
    let total = 0;
    for (const item of &readonly items) {
        items.push(1);
        total += item;
    }
    return total;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=5 column=9 span="items.push(1)" line_source="items.push(1);"
/// @diagnostic.related line=4 column=24 span="&readonly items" line_source="for (const item of &readonly items) {" message="borrow starts here"
"#,
    );
}

/// Pushing through a mutable borrow parameter conflicts with the readonly borrow its iteration holds.
#[test]
fn test_reject_pushing_through_a_mutable_borrow_parameter_while_iterating_it() {
    let session = TestSession::single(
        r#"
export function grow(items: &int64[]): int64 {
    let total = 0;
    for (const item of &readonly items) {
        items.push(1);
        total += item;
    }
    return total;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=5 column=9 span="items.push(1)" line_source="items.push(1);"
/// @diagnostic.related line=4 column=24 span="&readonly items" line_source="for (const item of &readonly items) {" message="borrow starts here"
"#,
    );
}

/// A provided optional field of an index-signature alias writes its case at the field's type.
#[test]
fn test_write_an_index_signature_alias_into_its_optional_field() {
    let session = TestSession::single(
        r#"
type Fields = { readonly [key: string]: int32 };

struct Entry {
    fields?: Fields | undefined;
    depth: int32;
}

function at(fields: Fields): Entry {
    return Entry { fields, depth: 1 };
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Replacing a unique pointer invalidates a readonly borrow of a Copy field behind it.
#[test]
fn test_reject_replacing_a_unique_pointer_under_a_borrow_behind_it() {
    let mut program = TestProgram::mir(
        r#"
type Item {
    value: int32;
}

type Holder {
    item: ref<Item, unique, mutable>;
}

function test(v0: Holder, v1: ref<Item, unique, mutable>): int32 {
    local l0: Holder

entry(v0: Holder, v1: ref<Item, unique, mutable>):
    store l0, v0
    v2: ref<int32, borrowed, 'frame, readonly> = address (*(l0).0).0
    store (l0).0, v1
    v3: int32 = load (*v2)
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:16:5
   │
13 │ entry(v0: Holder, v1: ref<Item, unique, mutable>):
14 │     store l0, v0
15 │     v2: ref<int32, borrowed, 'frame, readonly> = address (*(l0).0).0
   │     ---------------------------------------------------------------- borrow starts here
16 │     store (l0).0, v1
   │     ^^^^^^^^^^^^^^^^
17 │     v3: int32 = load (*v2)
18 │     return v3
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}

/// A call that may replace a global's unique pointer invalidates a readonly borrow behind it.
#[test]
fn test_reject_a_call_under_a_borrow_behind_the_unique_pointer_of_a_global() {
    let mut program = TestProgram::mir(
        r#"
type Item {
    value: int32;
}

type Holder {
    item: ref<Item, unique, mutable>;
}

global holder: Holder = zeroinit

external function touch(): void

function test(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, readonly> = address (*(@holder).0).0
    call touch(): () => void
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:17:5
   │
14 │ function test(): int32 {
15 │ entry:
16 │     v0: ref<int32, borrowed, 'static, readonly> = address (*(@holder).0).0
   │     ---------------------------------------------------------------------- borrow starts here
17 │     call touch(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^
18 │     v1: int32 = load (*v0)
19 │     return v1
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}

/// A Copy write through one parameter leaves a readonly loan of a non-Copy value through another live.
#[test]
fn test_allow_a_copy_write_beside_a_non_copy_loan_of_another_root() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: ref<int32, unique, mutable>;
}

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable>, v1: ref<Pair, borrowed, 'a, mutable>, v2: int32): int32 {
entry(v0: ref<Pair, borrowed, 'a, mutable>, v1: ref<Pair, borrowed, 'a, mutable>, v2: int32):
    v3: ref<Pair, borrowed, 'a, readonly> = address (*v1)
    store (*v0).0, v2
    v4: int32 = load (*v3).0
    return v4
}
"#,
    );

    program.assert_verified();
}

/// A write through a stored copy of a handle invalidates a borrow issued while its allocation was fresh.
#[test]
fn test_reject_a_write_through_a_stored_handle_under_a_borrow_of_an_escaped_allocation() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): int32 {
    local l0: ref<Box, managed, mutable, local>

entry:
    v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v1: ref<int32, borrowed, 'frame, immutable> = address (*v0).0
    store l0, v0
    v2: int32 = 1
    v3: ref<Box, managed, mutable, local> = load l0
    store (*v3).0, v2
    v4: int32 = load (*v1)
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:15:5
   │
 9 │ entry:
10 │     v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
11 │     v1: ref<int32, borrowed, 'frame, immutable> = address (*v0).0
   │     ------------------------------------------------------------- borrow starts here
12 │     store l0, v0
13 │     v2: int32 = 1
14 │     v3: ref<Box, managed, mutable, local> = load l0
15 │     store (*v3).0, v2
   │     ^^^^^^^^^^^^^^^^^
16 │     v4: int32 = load (*v1)
17 │     return v4
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}

/// A borrow issued in a loop after the allocation escaped on the previous iteration conflicts with writes through the stored handle.
#[test]
fn test_reject_a_write_through_a_stored_handle_under_a_borrow_after_a_loop_escape() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: boolean): int32 {
    local l0: ref<Box, managed, mutable, local>

entry(v0: boolean):
    v1: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v2: ref<Box, managed, mutable, local> = new.zeroed Box, local
    store l0, v2
    jump body

body:
    v3: ref<int32, borrowed, 'frame, immutable> = address (*v1).0
    v4: ref<Box, managed, mutable, local> = load l0
    v5: int32 = 1
    store (*v4).0, v5
    v6: int32 = load (*v3)
    store l0, v1
    branch v0 => body | done

done:
    return v6
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:19:5
   │
14 │
15 │ body:
16 │     v3: ref<int32, borrowed, 'frame, immutable> = address (*v1).0
   │     ------------------------------------------------------------- borrow starts here
17 │     v4: ref<Box, managed, mutable, local> = load l0
18 │     v5: int32 = 1
19 │     store (*v4).0, v5
   │     ^^^^^^^^^^^^^^^^^
20 │     v6: int32 = load (*v3)
21 │     store l0, v1
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}

/// A write through a handle stored in a global invalidates a borrow issued while its allocation was fresh.
#[test]
fn test_reject_a_write_through_a_global_handle_under_a_borrow_of_an_escaped_allocation() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

global slot: ref<Box, managed, mutable, local> = zeroinit

function test(): int32 {
entry:
    v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v1: ref<int32, borrowed, 'frame, immutable> = address (*v0).0
    store @slot, v0
    v2: int32 = 1
    v3: ref<Box, managed, mutable, local> = load @slot
    store (*v3).0, v2
    v4: int32 = load (*v1)
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:15:5
   │
 9 │ entry:
10 │     v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
11 │     v1: ref<int32, borrowed, 'frame, immutable> = address (*v0).0
   │     ------------------------------------------------------------- borrow starts here
12 │     store @slot, v0
13 │     v2: int32 = 1
14 │     v3: ref<Box, managed, mutable, local> = load @slot
15 │     store (*v3).0, v2
   │     ^^^^^^^^^^^^^^^^^
16 │     v4: int32 = load (*v1)
17 │     return v4
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}
