use crate::tests::{TestProgram, TestSession};

/// A borrow of a local cannot be returned.
#[test]
fn test_reject_frame_return() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(): ref<int32, borrowed, 'a, mutable> {
    local l0: Box
entry:
    v0: int32 = 0
    v1: Box = aggregate (v0)
    store l0, v1
    v2: ref<Box, borrowed, 'frame, mutable> = address l0
    v3: ref<int32, borrowed, 'frame, mutable> = address (*v2).0
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:14:5
   │
12 │     v2: ref<Box, borrowed, 'frame, mutable> = address l0
13 │     v3: ref<int32, borrowed, 'frame, mutable> = address (*v2).0
14 │     return v3
   │     ^^^^^^^^^
15 │ }
16 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A borrowed parameter returns at its declared lifetime.
#[test]
fn test_allow_parameter_return_when_declared() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable>): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>):
    return v0
}
"#,
    );

    program.assert_verified();
}

/// A borrow of a managed field returns at any declared lifetime.
#[test]
fn test_allow_managed_field_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'a>(v0: ref<User, managed, mutable, local>): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    return v1
}
"#,
    );

    program.assert_verified();
}

/// A borrow of a managed slice element returns at any declared lifetime.
#[test]
fn test_allow_managed_slice_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, managed, mutable, local>): ref<int32, borrowed, 'a, readonly> {
entry(v0: slice<int32, managed, mutable, local>):
    v1: int64 = 0
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0)[v1]
    return v2
}
"#,
    );

    program.assert_verified();
}

/// A borrow of a unique slice element dies with the frame that owns the slice.
#[test]
fn test_reject_unique_slice_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, unique, mutable>): ref<int32, borrowed, 'a, mutable> {
entry(v0: slice<int32, unique, mutable>):
    v1: int64 = 0
    v2: ref<int32, borrowed, 'frame, mutable> = address (*v0)[v1]
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
 ──▶ <test.tsppm>:6:5
  │
4 │     v1: int64 = 0
5 │     v2: ref<int32, borrowed, 'frame, mutable> = address (*v0)[v1]
6 │     return v2
  │     ^^^^^^^^^
7 │ }
8 │
  │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A borrow of a unique pointee dies with the frame that owns the pointer.
#[test]
fn test_reject_unique_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'a>(v0: ref<User, unique, mutable>): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<User, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:9:5
   │
 7 │ entry(v0: ref<User, unique, mutable>):
 8 │     v1: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
 9 │     return v1
   │     ^^^^^^^^^
10 │ }
11 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A borrow of managed storage is not static.
#[test]
fn test_reject_managed_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, local>): ref<int32, borrowed, 'static, readonly> {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:9:5
   │
 7 │ entry(v0: ref<User, managed, mutable, local>):
 8 │     v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
 9 │     return v1
   │     ^^^^^^^^^
10 │ }
11 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A frame borrow held in an aggregate is not static.
#[test]
fn test_reject_aggregate_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Box<'a> {
    value: ref<int32, borrowed, 'a, mutable>;
}

function test(v0: Box<'frame>): ref<int32, borrowed, 'static, mutable> {
entry(v0: Box<'frame>):
    v1: ref<int32, borrowed, 'frame, mutable> = field.get v0, 0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:9:5
   │
 7 │ entry(v0: Box<'frame>):
 8 │     v1: ref<int32, borrowed, 'frame, mutable> = field.get v0, 0
 9 │     return v1
   │     ^^^^^^^^^
10 │ }
11 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A field of an aggregate returns at the lifetime of its path.
#[test]
fn test_allow_aggregate_field_return_with_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: Pair<'a, 'b>): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: Pair<'a, 'b>):
    v3: ref<int32, borrowed, 'a, readonly> = field.get v2, 0
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A field of an aggregate cannot return at the lifetime of another path.
#[test]
fn test_reject_aggregate_field_return_with_wrong_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: Pair<'a, 'b>): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: Pair<'a, 'b>):
    v3: ref<int32, borrowed, 'b, readonly> = field.get v2, 1
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:10:5
   │
 8 │ entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: Pair<'a, '··
 9 │     v3: ref<int32, borrowed, 'b, readonly> = field.get v2, 1
10 │     return v3
   │     ^^^^^^^^^
11 │ }
12 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// An aggregate returns with each path at its declared lifetime.
#[test]
fn test_allow_aggregate_return_with_distinct_path_lifetimes() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>): Pair<'a, 'b> {
entry(v0: Pair<'a, 'b>):
    return v0
}
"#,
    );

    program.assert_verified();
}

/// An aggregate cannot return with its path lifetimes swapped.
#[test]
fn test_reject_aggregate_return_with_swapped_path_lifetimes() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>): Pair<'b, 'a> {
entry(v0: Pair<'a, 'b>):
    return v0
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.tsppm>:9:5
   │
 7 │ function test<'a, 'b>(v0: Pair<'a, 'b>): Pair<'b, 'a> {
 8 │ entry(v0: Pair<'a, 'b>):
 9 │     return v0
   │     ^^^^^^^^^
10 │ }
11 │
   │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A borrow read from a variant payload keeps the variant lifetime.
#[test]
fn test_reject_variant_borrow_return_as_static() {
    let mut program = TestProgram::mir(
        r#"
type Value<'a> = variant<uint8> { 0uint8 = ref<int32, borrowed, 'a, mutable>; 1uint8 = int32; };

function test<'a>(v0: Value<'a>): ref<int32, borrowed, 'static, mutable> {
entry(v0: Value<'a>):
    v1: ref<int32, borrowed, 'a, mutable> = variant.payload v0, 0
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
 ──▶ <test.tsppm>:7:5
  │
5 │ entry(v0: Value<'a>):
6 │     v1: ref<int32, borrowed, 'a, mutable> = variant.payload v0, 0
7 │     return v1
  │     ^^^^^^^^^
8 │ }
9 │
  │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A borrow of a readonly global is static.
#[test]
fn test_allow_static_borrow_return() {
    let mut program = TestProgram::mir(
        r#"
readonly global value: int32 = 1

function test(): ref<int32, borrowed, 'static, readonly> {
entry:
    v0: ref<int32, borrowed, 'static, readonly> = address @value
    v1: ref<int32, borrowed, 'static, readonly> = cast.bit v0 -> ref<int32, borrowed, 'static, readonly>
    return v1
}
"#,
    );

    program.assert_verified();
}

/// A borrowed parameter cannot return at the lifetime of another parameter.
#[test]
fn test_reject_wrong_parameter_lifetime_return() {
    let mut program = TestProgram::mir(
        r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>):
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
 ──▶ <test.tsppm>:4:5
  │
2 │ function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>):··
3 │ entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>):
4 │     return v1
  │     ^^^^^^^^^
5 │ }
6 │
  │

for more information about an error, run `tspp explain borrow-outlives-origin`
"#,
    );
}

/// A static borrow returns at any declared lifetime.
#[test]
fn test_allow_static_return_at_slot_result() {
    let mut program = TestProgram::mir(
        r#"
readonly global value: int32 = 1

function test<'L>(): ref<int32, borrowed, 'L, readonly> {
entry:
    v0: ref<int32, borrowed, 'static, readonly> = address @value
    return v0
}
"#,
    );

    program.assert_verified();
}

/// A parameter's storage dies with the frame, so its borrow cannot be static.
#[test]
fn test_reject_returning_a_borrow_of_a_parameter_as_static() {
    let session = TestSession::single(
        r#"
export function cplusplusMode(x: int32): &'static readonly int32 {
    return &readonly x;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=borrow-outlives-origin message="borrow does not live long enough"
/// @diagnostic.label line=3 column=5 span="return &readonly x" line_source="return &readonly x;"
"#,
    );
}

/// A temporary's borrow cannot leave the frame.
#[test]
fn test_reject_returning_a_borrow_of_a_temporary() {
    let session = TestSession::single(
        r#"
struct Foo {
    value: int32;
}

function id(foo: Foo): Foo {
    return foo;
}

export function fromTemporary(): &readonly int32 {
    const foo = &readonly id(Foo { value: 3 });
    return &readonly foo.value;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.tspp", r#"
/// @diagnostic.error id=borrow-outlives-origin message="borrow does not live long enough"
/// @diagnostic.label line=12 column=5 span="return &readonly foo.value" line_source="return &readonly foo.value;"
"#);
}

/// A borrow into managed storage lives in the managed extent, so a function returns it without inputs.
#[test]
fn test_allow_returning_a_borrow_into_managed_storage() {
    let session = TestSession::single(
        r#"
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function spawn(): &Player {
    let world = new World();
    return &world.player;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// A borrow into managed storage does not live in static storage.
#[test]
fn test_reject_a_borrow_into_managed_storage_as_static() {
    let session = TestSession::single(
        r#"
struct Player {
    score: int32;
}

class World {
    player: Player;

    constructor() {
        this.player = Player { score: 1 };
    }
}

function keep(world: World): &'static readonly Player {
    return &readonly world.player;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.tspp", r#"
/// @diagnostic.error id=borrow-outlives-origin message="borrow does not live long enough"
/// @diagnostic.label line=15 column=5 span="return &readonly world.player" line_source="return &readonly world.player;"
"#);
}

/// A unique allocation reinterpreted as a borrow leaks into the static region.
#[test]
fn test_allow_a_leaked_unique_allocation_as_a_static_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): ref<int32, borrowed, 'static, mutable> {
entry(v0: ref<int32, unique, mutable>):
    v1: ref<int32, borrowed, 'static, mutable> = intrinsic.memory.raw.transmute(v0)
    return v1
}
"#,
    );

    program.assert_verified();
}
