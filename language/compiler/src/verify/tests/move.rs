use crate::tests::{TestProgram, TestSession};

#[test]
fn test_reject_projected_move_use_after_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 0
    v5: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:11:5
   │
 8 │ entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
 9 │     v2: Pair = aggregate (v0, v1)
10 │     v3: ref<int32, unique, mutable> = field.get v2, 0
   │     ------------------------------------------------- value moved here
11 │     v4: ref<int32, unique, mutable> = field.get v2, 0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     v5: ref<int32, unique, mutable> = field.get v2, 1
13 │     return
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

#[test]
fn test_allow_complete_struct_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_complete_tuple_decomposition() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: { ref<int32, unique, mutable>, ref<int32, unique, mutable> } = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_complete_array_decomposition() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: [ref<int32, unique, mutable>; 2] = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = element.get v2, 0
    v4: ref<int32, unique, mutable> = element.get v2, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_complete_nested_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v6: ref<int32, unique, mutable> = field.get v4, 1
    v7: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_aggregate_move_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair): void {
entry(v0: Pair):
    jump next(v0)

next(v1: Pair):
    v2: ref<int32, unique, mutable> = field.get v1, 0
    v3: ref<int32, unique, mutable> = field.get v1, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_return() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_call() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_sibling_use_after_partial_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v1): (ref<int32, unique, mutable>) => void
    v2: ref<int32, unique, mutable> = field.get v0, 1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_reinitialization_after_partial_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v2: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v2): (ref<int32, unique, mutable>) => void
    v3: Pair = field.set v0, 0, v1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_across_branch() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    v3: Pair = aggregate (v0, v1)
    v4: ref<int32, unique, mutable> = field.get v3, 0
    branch v2 => b1 | b2

b1:
    return

b2:
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_partial_move_before_panic() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, managed, readonly>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, managed, readonly>):
    v3: Pair = aggregate (v0, v1)
    v4: ref<int32, unique, mutable> = field.get v3, 0
    panic v2
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_variant_use_after_payload_move() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0
    v2: uint8 = variant.tag v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
 ──▶ <test.dsm>:7:5
  │
4 │ function test(v0: Value): void {
5 │ entry(v0: Value):
6 │     v1: ref<int32, unique, mutable> = variant.payload v0, 0
  │     ------------------------------------------------------- value moved here
7 │     v2: uint8 = variant.tag v0
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^
8 │     return
9 │ }
  │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

/// A field loaded out of a local through its address moves that field alone.
#[test]
fn test_allow_a_field_move_out_of_a_local_through_its_address() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair): void {
    local l0: Pair

entry(v0: Pair):
    local.set l0, v0
    v1: ref<Pair, borrowed, exclusive, frame> = local.project l0
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.project v1, 0
    v3: ref<int32, unique, mutable> = load v2
    v4: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.project v1, 1
    v5: ref<int32, unique, mutable> = load v4
    return
}
"#,
    );

    program.assert_verified();
}

/// A whole read of a local after one of its fields moved out is a use after move.
#[test]
fn test_reject_a_whole_read_of_a_local_after_a_field_moved_out() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair): Pair {
    local l0: Pair

entry(v0: Pair):
    local.set l0, v0
    v1: ref<Pair, borrowed, exclusive, frame> = local.project l0
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.project v1, 0
    v3: ref<int32, unique, mutable> = load v2
    v4: Pair = local.get l0
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:15:5
   │
12 │     v1: ref<Pair, borrowed, exclusive, frame> = local.project l0
13 │     v2: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.project v1, 0
14 │     v3: ref<int32, unique, mutable> = load v2
   │     ----------------------------------------- value moved here
15 │     v4: Pair = local.get l0
   │     ^^^^^^^^^^^^^^^^^^^^^^^
16 │     return v4
17 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

/// A store into a moved field reinitializes it, making the local whole again.
#[test]
fn test_allow_a_store_reinitializing_a_moved_field() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): Pair {
    local l0: Pair

entry(v0: Pair, v1: ref<int32, unique, mutable>):
    local.set l0, v0
    v2: ref<Pair, borrowed, exclusive, frame> = local.project l0
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.project v2, 0
    v4: ref<int32, unique, mutable> = load v3
    store v3, v1
    v5: Pair = local.get l0
    return v5
}
"#,
    );

    program.assert_verified();
}

/// A field moves out of an owned value.
#[test]
fn test_allow_a_field_move_out_of_an_owned_value() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&exclusive this): void {}
}

struct Buffer {
    token: Token;
}

function toToken(buffer: Buffer): Token {
    return buffer.token;
}

export function main(): void {
    let buffer = Buffer { token: Token { id: 1 } };
    let token = toToken(buffer);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// Moves and borrows track fields: a moved field stays moved, a borrowed field cannot move, sibling fields stay disjoint.
#[test]
fn test_track_moves_and_borrows_per_field() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&exclusive this): void {}
}

struct Pair {
    a: int32;
    b: Token;
}

function consume(token: Token): void {}

function read(id: &readonly int32): void {}

function readToken(token: &readonly Token): void {}

export function derefAfterMove(): void {
    const x = Pair { a: 1, b: Token { id: 2 } };
    consume(x.b);
    read(&readonly x.b.id);
}

export function borrowAfterMove(): void {
    const x = Pair { a: 1, b: Token { id: 2 } };
    consume(x.b);
    const p = &readonly x.b;
    readToken(p);
}

export function moveAfterBorrow(): void {
    const x = Pair { a: 1, b: Token { id: 2 } };
    const p = &readonly x.b;
    consume(x.b);
    readToken(p);
}

export function mutBorrowAfterMutBorrow(): void {
    let x = Pair { a: 1, b: Token { id: 2 } };
    const p = &exclusive x.a;
    const q = &exclusive x.a;
    read(p);
    read(q);
}

export function disjointFields(): void {
    let x = Pair { a: 1, b: Token { id: 2 } };
    const p = &exclusive x.a;
    const q = &exclusive x.b;
    read(p);
    readToken(q);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=22 column=10 span="&readonly x.b.id" line_source="read(&readonly x.b.id);"
/// @diagnostic.related line=21 column=13 span="x.b" line_source="consume(x.b);" message="value moved here"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=28 column=15 span="&readonly x.b" line_source="const p = &readonly x.b;"
/// @diagnostic.related line=27 column=13 span="x.b" line_source="consume(x.b);" message="value moved here"
/// @diagnostic.error id=invalidation-of-borrowed-place message="cannot invalidate borrowed place"
/// @diagnostic.label line=35 column=13 span="x.b" line_source="consume(x.b);"
/// @diagnostic.related line=34 column=15 span="&readonly x.b" line_source="const p = &readonly x.b;" message="borrow starts here"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=42 column=15 span="&exclusive x.a" line_source="const q = &exclusive x.a;"
/// @diagnostic.related line=41 column=15 span="&exclusive x.a" line_source="const p = &exclusive x.a;" message="borrow starts here"
"#);
}
