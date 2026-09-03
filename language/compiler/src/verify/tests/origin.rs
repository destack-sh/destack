use crate::tests::TestProgram;

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

    program.assert_verify_errors(r#"
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
"#);
}

#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'a>(v0: ref<User, managed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<User, managed, 'a, mutable>, v1: boolean):
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
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

#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, managed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: slice<int32, managed, 'a, mutable>, v1: boolean):
    v2: int64 = 0
    v3: ref<int32, borrowed, readonly> = element.address v0, v2
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

b3(v3: ref<int32, borrowed, mutable>):
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
12 │ b3(v3: ref<int32, borrowed, mutable>):
13 │     return v3
   │     ^^^^^^^^^
14 │ }
15 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

#[test]
fn test_allow_handle_borrow_through_a_local_round_trip() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

@copy
type Box {
    message: ref<String, managed, mutable, undefined, local>;
}

type Maybe = variant<uint1> { 0uint1 = ref<String, managed, readonly, local>; 1uint1 = void; }

constant string.0: String = "left"

function test<'a>(v0: ref<Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: Maybe
    local l1: ref<String, borrowed, 'a, readonly, local>, readonly

entry(v0: ref<Box, borrowed, 'a, readonly, local>):
    v1: ref<ref<String, managed, readonly, undefined, local>, borrowed, readonly, local> = field.address v0, 0
    v2: ref<String, managed, readonly, undefined, local> = load v1
    v3: ref<String, managed, readonly, undefined, local> = undefined
    v4: boolean = eq v2, v3
    branch v4 => b2 | b1

b1:
    v5: ref<String, managed, readonly, local> = cast.bit v2 -> ref<String, managed, readonly, local>
    v6: Maybe = variant.new 0, v5
    local.set l0, v6
    jump b3

b2:
    v8: Maybe = variant.new 1
    local.set l0, v8
    jump b3

b3:
    v9: Maybe = local.get l0
    variant.switch v9, 0 => b5, 1 => b6

b4:
    v14: ref<String, borrowed, 'a, readonly, local> = local.get l1
    return v14

b5:
    v10: ref<String, managed, readonly, local> = variant.payload v9, 0
    v11: ref<String, borrowed, 'a, readonly, local> = cast.bit v10 -> ref<String, borrowed, 'a, readonly, local>
    local.set l1, v11
    jump b4

b6:
    v12: ref<String, managed, mutable, local> = global.address string.0
    v13: ref<String, borrowed, 'a, readonly, local> = cast.bit v12 -> ref<String, borrowed, 'a, readonly, local>
    local.set l1, v13
    jump b4
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_handle_borrow_through_a_borrowed_receiver() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

type Box {
    message: ref<String, managed, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
entry(v0: ref<Box, borrowed, 'a, readonly, local>):
    v1: ref<ref<String, managed, readonly, local>, borrowed, readonly, local> = field.address v0, 0
    v2: ref<String, managed, readonly, local> = load v1
    v3: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readonly, local>
    return v3
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_handle_borrow_through_a_copied_aggregate() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

@copy
type Box {
    message: ref<String, managed, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
entry(v0: ref<Box, borrowed, 'a, readonly, local>):
    v1: Box = load v0
    v2: ref<String, managed, mutable, local> = field.get v1, 0
    v3: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readonly, local>
    return v3
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_handle_borrow_beside_a_sibling_field_store() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

type Pair {
    message: ref<String, managed, mutable, local>;
    other: ref<String, managed, mutable, local>;
}

constant string.0: String = "next"

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable, local>): ref<String, borrowed, 'a, readonly, local> {
entry(v0: ref<Pair, borrowed, 'a, mutable, local>):
    v1: ref<ref<String, managed, mutable, local>, borrowed, mutable, local> = field.address v0, 0
    v2: ref<String, managed, mutable, local> = load v1
    v3: ref<ref<String, managed, mutable, local>, borrowed, mutable, local> = field.address v0, 1
    v4: ref<String, managed, mutable, local> = global.address string.0
    store v3, v4
    v5: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readonly, local>
    return v5
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_a_handle_borrow_after_its_slot_is_overwritten() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

type Box {
    message: ref<String, managed, mutable, local>;
}

constant string.0: String = "next"

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): ref<String, borrowed, 'a, readonly, local> {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<ref<String, managed, mutable, local>, borrowed, mutable, local> = field.address v0, 0
    v2: ref<String, managed, mutable, local> = load v1
    v3: ref<String, managed, mutable, local> = global.address string.0
    store v1, v3
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readonly, local>
    return v4
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_handle_borrow_from_a_handle_parameter() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

function test<'a>(v0: ref<String, managed, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, local> {
    local l0: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v2: ref<String, managed, mutable, local> = local.get l0
    v3: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readonly, local>
    return v3
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:13:5
   │
11 │     v2: ref<String, managed, mutable, local> = local.get l0
12 │     v3: ref<String, borrowed, 'a, readonly, local> = cast.bit v2 -> ref<String, borrowed, 'a, readon··
13 │     return v3
   │     ^^^^^^^^^
14 │ }
15 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

#[test]
fn test_allow_null_borrow_at_any_region() {
    let mut program = TestProgram::mir(
        r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): ref<String, borrowed, 'a, readonly, undefined, local> {
entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    v1: ref<String, borrowed, 'a, readonly, undefined, local> = undefined
    return v1
}
"#,
    );

    program.assert_verified();
}
