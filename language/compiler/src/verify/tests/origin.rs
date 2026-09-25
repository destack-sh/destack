use crate::tests::{TestProgram, TestSession};

/// A select of two borrows keeps both lifetimes.
#[test]
fn test_reject_selected_borrow_with_wrong_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean):
    v3: ref<int32, borrowed, 'a, readonly> = select v2, v0, v1
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
 ──▶ <test.dsm>:5:5
  │
3 │ entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean):
4 │     v3: ref<int32, borrowed, 'a, readonly> = select v2, v0, v1
5 │     return v3
  │     ^^^^^^^^^
6 │ }
7 │
  │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

/// A managed field borrow passes through block parameters at any lifetime.
#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'a>(v0: ref<User, managed, mutable, local>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<User, managed, mutable, local>, v1: boolean):
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    branch v1 => b1(v2) | b2(v2)

b1(v3: ref<int32, borrowed, 'a, readonly>):
    return v3

b2(v4: ref<int32, borrowed, 'a, readonly>):
    return v4
}
"#,
    );

    program.assert_verified();
}

/// A managed slice element borrow passes through block parameters at any lifetime.
#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, managed, mutable, local>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: slice<int32, managed, mutable, local>, v1: boolean):
    v2: int64 = 0
    v3: ref<int32, borrowed, 'managed, readonly> = address (*v0)[v2]
    branch v1 => b1(v3) | b2(v3)

b1(v4: ref<int32, borrowed, 'a, readonly>):
    return v4

b2(v5: ref<int32, borrowed, 'a, readonly>):
    return v5
}
"#,
    );

    program.assert_verified();
}

/// A block parameter carries the lifetime of its incoming borrow.
#[test]
fn test_carry_lifetime_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, borrowed, 'a, mutable>):
    return v2

b2(v3: ref<int32, borrowed, 'a, mutable>):
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A block parameter carries the lifetime of each aggregate path.
#[test]
fn test_carry_aggregate_path_lifetimes_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: boolean): ref<int32, borrowed, 'b, readonly> {
entry(v0: Pair<'a, 'b>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: Pair<'a, 'b>):
    jump b3(v2)

b2(v3: Pair<'a, 'b>):
    jump b3(v3)

b3(v4: Pair<'a, 'b>):
    v5: ref<int32, borrowed, 'b, readonly> = field.get v4, 1
    return v5
}
"#,
    );

    program.assert_verified();
}

/// A field write may store a longer borrow under an outlives bound.
#[test]
fn test_allow_field_write_satisfying_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void where 'a: 'b {
entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
    v2: Pair<'a, 'b> = field.set v0, 1, v1
    return
}
"#,
    );

    program.assert_verified();
}

/// A field write cannot store a borrow of an unrelated lifetime.
#[test]
fn test_reject_field_write_violating_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void {
entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
    v2: Pair<'a, 'b> = field.set v0, 1, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:9:5
   │
 7 │ function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void {
 8 │ entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
 9 │     v2: Pair<'a, 'b> = field.set v0, 1, v1
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

/// A join merges the lifetimes of its incoming borrows.
#[test]
fn test_merge_lifetimes_at_join() {
    let mut program = TestProgram::mir(
        r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>, v2: boolean): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>, v2: boolean):
    branch v2 => b1 | b2

b1:
    jump b3(v0)

b2:
    jump b3(v1)

b3(v3: ref<int32, borrowed, 'a | 'b, mutable>):
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:13:5
   │
11 │
12 │ b3(v3: ref<int32, borrowed, 'a | 'b, mutable>):
13 │     return v3
   │     ^^^^^^^^^
14 │ }
15 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

/// A borrow cast from a handle round-tripped through a local returns at the declared lifetime.
#[test]
fn test_allow_handle_borrow_through_a_local_round_trip() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type Stored = variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }

type Box {
    message: Stored;
}

type Maybe = variant<uint1> { 0uint1 = void; 1uint1 = ref<String, managed, readonly, local>; }

constant string.0: String = "left"

function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: Maybe
    local l1: ref<String, borrowed, 'a, readonly>, readonly

entry(v0: ref<Box, borrowed, 'a, readonly>):
    v1: ref<Stored, borrowed, 'a, readonly> = address (*v0).0
    v2: Stored = load (*v1)
    v3: uint1 = variant.tag v2
    v15: uint1 = 1
    v4: boolean = eq v3, v15
    branch v4 => b2 | b1

b1:
    v16: ref<String, managed, mutable, local> = variant.payload v2, 0
    v5: ref<String, managed, readonly, local> = cast.bit v16 -> ref<String, managed, readonly, local>
    v6: Maybe = variant.new 1, v5
    store l0, v6
    jump b3

b2:
    v8: Maybe = variant.new 0
    store l0, v8
    jump b3

b3:
    v9: Maybe = load l0
    variant.switch v9, 1 => b5, 0 => b6

b4:
    v14: ref<String, borrowed, 'a, readonly> = load l1
    return v14

b5:
    v10: ref<String, managed, readonly, local> = variant.payload v9, 1
    v11: ref<String, borrowed, 'a, readonly> = cast.bit v10 -> ref<String, borrowed, 'a, readonly>
    store l1, v11
    jump b4

b6:
    v12: ref<String, managed, mutable, local> = address @string.0
    v13: ref<String, borrowed, 'a, readonly> = cast.bit v12 -> ref<String, borrowed, 'a, readonly>
    store l1, v13
    jump b4
}
"#,
    );

    program.assert_verified();
}

/// A borrow cast from a handle read through a receiver returns at the receiver lifetime.
#[test]
fn test_allow_handle_borrow_through_a_borrowed_receiver() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type Box {
    message: ref<String, managed, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
entry(v0: ref<Box, borrowed, 'a, readonly>):
    v1: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly> = address (*v0).0
    v2: ref<String, managed, readonly, local> = load (*v1)
    v3: ref<String, borrowed, 'a, readonly> = cast.bit v2 -> ref<String, borrowed, 'a, readonly>
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A borrow cast from a handle in a copied aggregate returns at the source lifetime.
#[test]
fn test_allow_handle_borrow_through_a_copied_aggregate() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type Box {
    message: ref<String, managed, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
entry(v0: ref<Box, borrowed, 'a, readonly>):
    v1: Box = load (*v0)
    v2: ref<String, managed, mutable, local> = field.get v1, 0
    v3: ref<String, borrowed, 'a, readonly> = cast.bit v2 -> ref<String, borrowed, 'a, readonly>
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A store to a sibling field leaves a borrow cast from a handle valid.
#[test]
fn test_allow_handle_borrow_beside_a_sibling_field_store() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type Pair {
    message: ref<String, managed, mutable, local>;
    other: ref<String, managed, mutable, local>;
}

constant string.0: String = "next"

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable>): ref<String, borrowed, 'a, readonly> {
entry(v0: ref<Pair, borrowed, 'a, mutable>):
    v1: ref<ref<String, managed, mutable, local>, borrowed, 'a, mutable> = address (*v0).0
    v2: ref<String, managed, mutable, local> = load (*v1)
    v3: ref<ref<String, managed, mutable, local>, borrowed, 'a, mutable> = address (*v0).1
    v4: ref<String, managed, mutable, local> = address @string.0
    store (*v3), v4
    v5: ref<String, borrowed, 'a, readonly> = cast.bit v2 -> ref<String, borrowed, 'a, readonly>
    return v5
}
"#,
    );

    program.assert_verified();
}

/// A borrow cast from a handle survives an overwrite of the slot the handle came from.
#[test]
fn test_allow_a_handle_borrow_after_its_slot_is_overwritten() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type Box {
    message: ref<String, managed, mutable, local>;
}

constant string.0: String = "next"

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): ref<String, borrowed, 'a, readonly> {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<ref<String, managed, mutable, local>, borrowed, 'a, mutable> = address (*v0).0
    v2: ref<String, managed, mutable, local> = load (*v1)
    v3: ref<String, managed, mutable, local> = address @string.0
    store (*v1), v3
    v4: ref<String, borrowed, 'a, readonly> = cast.bit v2 -> ref<String, borrowed, 'a, readonly>
    return v4
}
"#,
    );

    program.assert_verified();
}

/// A borrow of a handle is returned at any region, managed storage living while the borrow is held.
#[test]
fn test_allow_a_handle_borrow_returned_at_any_region() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

function test<'a>(v0: ref<String, managed, mutable, local>, v1: ref<int32, borrowed, 'a, readonly>): ref<String, borrowed, 'a, readonly> {
    local l0: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<int32, borrowed, 'a, readonly>):
    store l0, v0
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, borrowed, 'a, readonly> = cast.bit v2 -> ref<String, borrowed, 'a, readonly>
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A variant case without a borrow returns at any lifetime.
#[test]
fn test_allow_null_borrow_at_any_region() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

function test<'a>(v0: ref<int32, borrowed, 'a, readonly>): variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; } {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; } = variant.new 0
    return v1
}
"#,
    );

    program.assert_verified();
}

/// Wrapping a managed string in a nested newtype keeps the handle's origin.
#[test]
fn test_wrap_a_managed_string_in_a_nested_newtype() {
    let session = TestSession::single(
        r#"
newtype Id = string;
newtype Ref = Id;

function map<T, U>(value: T, transform: (value: T) => U): U {
    transform(value)
}

export function wrap(id: Id): Ref {
    map(id, (id) => Ref(id))
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Id = newtype<String>;

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type test.main.Ref = newtype<test.main.Id>;

function test.main.wrap(v0: ref<test.main.Id, managed, mutable, local>): ref<test.main.Ref, managed, mutable, local> {
    local l0: ref<test.main.Id, managed, mutable, local>

entry(v0: ref<test.main.Id, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Id, managed, mutable, local> = load l0
    v2: ptr<void, readonly> = null
    v3: function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.wrap.closure#0, v2
    v4: ref<test.main.Ref, managed, mutable, local> = call test.main.map<ref<test.main.Id, managed, mutable, local>, ref<test.main.Ref, managed, mutable, local>>(v1, v3): (ref<test.main.Id, managed, mutable, local>, function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>
    return v4
}

function test.main.map<T, U>(v0: T, v1: function<(T) => U, repeatable, managed, mutable, local>): U {
    local l0: T
    local l1: function<(T) => U, repeatable, managed, mutable, local>

entry(v0: T, v1: function<(T) => U, repeatable, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: function<(T) => U, repeatable, managed, mutable, local> = load l1
    v3: T = load l0
    v4: function<(T) => U, repeatable, borrowed, 'managed, mutable> = cast.bit v2 -> function<(T) => U, repeatable, borrowed, 'managed, mutable>
    v5: U = call.indirect v4(v3): (T) => U
    return v5
}

shared function test.main.map<ref<test.main.Id, managed, mutable, local>, ref<test.main.Ref, managed, mutable, local>>(v0: ref<test.main.Id, managed, mutable, local>, v1: function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local>): ref<test.main.Ref, managed, mutable, local>;

function test.main.wrap.closure#0(v0: ref<test.main.Id, managed, mutable, local>): ref<test.main.Ref, managed, mutable, local> {
    local l0: ref<test.main.Id, managed, mutable, local>

entry(v0: ref<test.main.Id, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Id, managed, mutable, local> = load l0
    v2: ref<test.main.Ref, managed, mutable, local> = cast.bit v1 -> ref<test.main.Ref, managed, mutable, local>
    return v2
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@6 size=16 align=8
/// @layout.field owner=type@6 index=0 name=codeUnits offset=0 size=16 align=8
"#);
    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A closure created under a borrowing function passes as a plain function value.
#[test]
fn test_pass_a_closure_created_under_a_borrowing_function() {
    let session = TestSession::single(
        r#"
newtype Id = string;
newtype Ref = Id;

function map<T, U>(value: T, transform: (value: T) => U): U {
    transform(value)
}

export function wrap(id: Id, label: &readonly string): Ref {
    map(id, (id) => Ref(id))
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
    session.assert_mir_lowered("main.ds", r#"
type test.main.Id = newtype<String>;

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type test.main.Ref = newtype<test.main.Id>;

function test.main.wrap<'a>(v0: ref<test.main.Id, managed, mutable, local>, v1: ref<String, borrowed, 'a, readonly>): ref<test.main.Ref, managed, mutable, local> {
    local l0: ref<test.main.Id, managed, mutable, local>
    local l1: ref<String, borrowed, 'a, readonly>

entry(v0: ref<test.main.Id, managed, mutable, local>, v1: ref<String, borrowed, 'a, readonly>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Id, managed, mutable, local> = load l0
    v3: ptr<void, readonly> = null
    v4: function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.wrap.closure#0, v3
    v5: ref<test.main.Ref, managed, mutable, local> = call test.main.map<ref<test.main.Id, managed, mutable, local>, ref<test.main.Ref, managed, mutable, local>>(v2, v4): (ref<test.main.Id, managed, mutable, local>, function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>
    return v5
}

function test.main.map<T, U>(v0: T, v1: function<(T) => U, repeatable, managed, mutable, local>): U {
    local l0: T
    local l1: function<(T) => U, repeatable, managed, mutable, local>

entry(v0: T, v1: function<(T) => U, repeatable, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: function<(T) => U, repeatable, managed, mutable, local> = load l1
    v3: T = load l0
    v4: function<(T) => U, repeatable, borrowed, 'managed, mutable> = cast.bit v2 -> function<(T) => U, repeatable, borrowed, 'managed, mutable>
    v5: U = call.indirect v4(v3): (T) => U
    return v5
}

shared function test.main.map<ref<test.main.Id, managed, mutable, local>, ref<test.main.Ref, managed, mutable, local>>(v0: ref<test.main.Id, managed, mutable, local>, v1: function<(ref<test.main.Id, managed, mutable, local>) => ref<test.main.Ref, managed, mutable, local>, repeatable, managed, mutable, local>): ref<test.main.Ref, managed, mutable, local>;

function test.main.wrap.closure#0<'a>(v0: ref<test.main.Id, managed, mutable, local>): ref<test.main.Ref, managed, mutable, local> {
    local l0: ref<test.main.Id, managed, mutable, local>

entry(v0: ref<test.main.Id, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Id, managed, mutable, local> = load l0
    v2: ref<test.main.Ref, managed, mutable, local> = cast.bit v1 -> ref<test.main.Ref, managed, mutable, local>
    return v2
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@6 size=16 align=8
/// @layout.field owner=type@6 index=0 name=codeUnits offset=0 size=16 align=8
"#);
}
