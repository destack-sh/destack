use crate::tests::{TestProgram, TestSession};

/// Consuming an owner invalidates a borrow a block parameter carries into it.
#[test]
fn test_reject_move_while_borrowed_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function consume(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}

function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
entry(v0: ref<Box, unique, mutable>, v1: boolean):
    v2: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
    branch v1 => b1(v2) | b2

b1(v3: ref<int32, borrowed, 'frame, mutable>):
    call consume(v0): (ref<Box, unique, mutable>) => void
    v4: int32 = load (*v3)
    return

b2:
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.tsppm>:17:5
   │
11 │ function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
12 │ entry(v0: ref<Box, unique, mutable>, v1: boolean):
13 │     v2: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
   │     ----------------------------------------------------------- borrow starts here
14 │     branch v1 => b1(v2) | b2
15 │
16 │ b1(v3: ref<int32, borrowed, 'frame, mutable>):
17 │     call consume(v0): (ref<Box, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
18 │     v4: int32 = load (*v3)
19 │     return
   │

for more information about an error, run `tspp explain invalidation-of-borrowed-place`
"#,
    );
}

/// Reading storage a call consumed is a use after move.
#[test]
fn test_reject_use_after_call_move() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    call consume(v0): (ref<int32, unique, mutable>) => void
    v1: int32 = load (*v0)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.tsppm>:10:5
   │
 7 │ function test(v0: ref<int32, unique, mutable>): void {
 8 │ entry(v0: ref<int32, unique, mutable>):
 9 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ------------------------------------------------------- value moved here
10 │     v1: int32 = load (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `tspp explain use-after-move`
"#,
    );
}

/// Reading a value an aggregate consumed is a use after move.
#[test]
fn test_reject_use_after_struct_move() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    v2: int32 = load (*v0)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.tsppm>:9:5
   │
 6 │ function test(v0: ref<int32, unique, mutable>): void {
 7 │ entry(v0: ref<int32, unique, mutable>):
 8 │     v1: Box = aggregate (v0)
   │     ------------------------ value moved here
 9 │     v2: int32 = load (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `tspp explain use-after-move`
"#,
    );
}

/// Completing one allocation twice is a use after move.
#[test]
fn test_reject_use_after_new_complete() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): void {
entry:
    v0: uninit<ref<Box, managed, mutable, local>> = new.uninit Box, local
    v1: ref<Box, managed, mutable, local> = new.complete v0
    v2: ref<Box, managed, mutable, local> = new.complete v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.tsppm>:10:5
   │
 7 │ entry:
 8 │     v0: uninit<ref<Box, managed, mutable, local>> = new.uninit Box, local
 9 │     v1: ref<Box, managed, mutable, local> = new.complete v0
   │     ------------------------------------------------------- value moved here
10 │     v2: ref<Box, managed, mutable, local> = new.complete v0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `tspp explain use-after-move`
"#,
    );
}

/// Reading a value moved on one branch is a conditional use after move.
#[test]
fn test_reject_maybe_moved_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
    jump b3

b2:
    jump b3

b3:
    v2: int32 = load (*v0)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.tsppm>:19:5
   │
10 │
11 │ b1:
12 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ------------------------------------------------------- value moved on this path
13 │     jump b3
14 │
15 │ b2:
16 │     jump b3
17 │
18 │ b3:
19 │     v2: int32 = load (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^
20 │     return
21 │ }
   │

for more information about an error, run `tspp explain maybe-use-after-move`
"#,
    );
}

/// Reading a value one incoming edge moved is a conditional use after move.
#[test]
fn test_reject_edge_dependent_move_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    branch v2 => done(v0) | done(v1)

done(v3: ref<int32, unique, mutable>):
    call consume(v3): (ref<int32, unique, mutable>) => void
    call consume(v0): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.tsppm>:13:5
   │
 7 │ function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
 8 │ entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
 9 │     branch v2 => done(v0) | done(v1)
   │     -------------------------------- value moved on this path
10 │
11 │ done(v3: ref<int32, unique, mutable>):
12 │     call consume(v3): (ref<int32, unique, mutable>) => void
13 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
14 │     return
15 │ }
   │

for more information about an error, run `tspp explain maybe-use-after-move`
"#,
    );
}

/// A constructor returning with a field unwritten is rejected.
#[test]
fn test_reject_constructor_return_with_uninitialized_field() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

constructor construct<'a>(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[field-left-uninitialized]: constructor returns before initializing field 1
  ──▶ <test.tsppm>:11:5
   │
 9 │     v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
10 │     store (*v2), v1
11 │     return
   │     ^^^^^^
12 │ }
13 │
   │

for more information about an error, run `tspp explain field-left-uninitialized`
"#,
    );
}

/// A constructor writing one field twice is rejected.
#[test]
fn test_reject_constructor_field_initialized_twice() {
    let mut program = TestProgram::mir(
        r#"
type Single {
    value: int32;
}

constructor construct<'a>(v0: ref<uninit<Single>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Single>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    v3: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v3), v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[field-initialized-twice]: constructor initializes field 0 twice
  ──▶ <test.tsppm>:11:5
   │
 9 │     store (*v2), v1
10 │     v3: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
11 │     store (*v3), v1
   │     ^^^^^^^^^^^^^^^
12 │     return
13 │ }
   │

for more information about an error, run `tspp explain field-initialized-twice`
"#,
    );
}

/// A reinitialized binding moves again only once.
#[test]
fn test_reject_a_use_after_the_second_move_of_a_reinitialized_binding() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

function consume(token: Token): void {}

export function main(): void {
    let x = Token { id: 0 };
    const u = x;
    x = Token { id: 1 };
    consume(x);
    consume(x);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.tspp", r#"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=15 column=13 span="x" line_source="consume(x);"
/// @diagnostic.related line=14 column=13 span="x" line_source="consume(x);" message="value moved here"
"#);
}

/// Read constructed storage after explicitly asserting initialization.
#[test]
fn test_read_completed_constructor_storage() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

constructor build<'a>(v0: ref<uninit<Box>, borrowed, 'a, exclusive>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<uninit<Box>, borrowed, 'a, exclusive>, v1: ref<int32, unique, mutable>):
    v2: ref<uninit<ref<int32, unique, mutable>>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    return
}

function test(v0: ref<int32, unique, mutable>): Box {
    local l0: Box

entry(v0: ref<int32, unique, mutable>):
    v1: ref<Box, borrowed, 'frame, exclusive> = address l0
    v2: ref<uninit<Box>, borrowed, 'frame, exclusive> = cast.bit v1 -> ref<uninit<Box>, borrowed, 'frame, exclusive>
    call build(v2, v0): <'a>(ref<uninit<Box>, borrowed, 'a, exclusive>, ref<int32, unique, mutable>) => void
    v3: Box = load l0
    return v3
}
"#,
    );

    program.assert_verified();
}

/// Passing the receiver out before every field is written is rejected.
#[test]
fn test_reject_this_escaping_before_every_field_initializes() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

function observe(v0: ref<Pair, managed, mutable, local>): void {
entry(v0: ref<Pair, managed, mutable, local>):
    return
}

constructor construct(v0: ref<uninit<Pair>, borrowed, 'managed, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, 'managed, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'managed, mutable> = address (*v0).0
    store (*v2), v1
    v5: ref<uninit<Pair>, borrowed, 'managed, exclusive> = address (*v0)
    v3: ref<Pair, managed, mutable, local> = cast.bit v5 -> ref<Pair, managed, mutable, local>
    call observe(v3): (ref<Pair, managed, mutable, local>) => void
    v4: ref<uninit<int32>, borrowed, 'managed, mutable> = address (*v0).1
    store (*v4), v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[receiver-before-initialization]: 'this' escapes before every field initializes
  ──▶ <test.tsppm>:18:5
   │
16 │     v5: ref<uninit<Pair>, borrowed, 'managed, exclusive> = address (*v0)
17 │     v3: ref<Pair, managed, mutable, local> = cast.bit v5 -> ref<Pair, managed, mutable, local>
18 │     call observe(v3): (ref<Pair, managed, mutable, local>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
19 │     v4: ref<uninit<int32>, borrowed, 'managed, mutable> = address (*v0).1
20 │     store (*v4), v1
   │

for more information about an error, run `tspp explain receiver-before-initialization`
"#,
    );
}

/// Passing the receiver out after every field is written is allowed.
#[test]
fn test_read_this_after_every_field_initializes() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

function observe(v0: ref<Pair, managed, mutable, local>): void {
entry(v0: ref<Pair, managed, mutable, local>):
    return
}

constructor construct(v0: ref<uninit<Pair>, borrowed, 'managed, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, 'managed, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'managed, mutable> = address (*v0).0
    store (*v2), v1
    v3: ref<uninit<int32>, borrowed, 'managed, mutable> = address (*v0).1
    store (*v3), v1
    v4: ref<Pair, managed, mutable, local> = cast.bit v0 -> ref<Pair, managed, mutable, local>
    call observe(v4): (ref<Pair, managed, mutable, local>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// A derived constructor with no fields of its own delegates to its base.
#[test]
fn test_delegate_to_a_base_constructor_from_a_derived_class_without_own_fields() {
    let mut program = TestProgram::mir(
        r#"
type Base {
    first: int32;
}

type Derived {
    first: int32;
}

constructor construct_base<'a>(v0: ref<uninit<Base>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Base>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    return
}

constructor construct<'a>(v0: ref<uninit<Derived>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Derived>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<Base>, borrowed, 'a, exclusive> = cast.bit v0 -> ref<uninit<Base>, borrowed, 'a, exclusive>
    call construct_base(v2, v1): <'a>(ref<uninit<Base>, borrowed, 'a, exclusive>, int32) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// A constructor initializes every field through its reborrowed receiver.
#[test]
fn test_initialize_every_field_through_the_reborrowed_receiver() {
    let session = TestSession::single(
        r#"
import { Cell } from "tspp:memory";

struct Next<N> {
    value: N | undefined;
}

struct Return<R> {
    value: R;
}

type Operation<R, N> = Next<N> | Return<R>;

struct Resolvers<T: Copy> {
    promise: Cell<T>;
    resolve: (value: T) => void;
}

class Request<Y: Copy, R: Copy, N> {
    private readonly operation: Cell<Operation<R, N> | undefined>;
    settlement: Resolvers<Y>;
    next: Cell<Request<Y, R, N> | undefined>;

    constructor(operation: Operation<R, N>, settlement: Resolvers<Y>) {
        this.operation = Cell.new(operation);
        this.settlement = settlement;
        this.next = Cell.new(undefined);
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

/// A call filling an uninitialized scalar field in place initializes that field.
#[test]
fn test_initialize_a_scalar_field_constructed_in_place() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

external function fill<'a>(ref<uninit<int32>, borrowed, 'a, exclusive>): void

constructor construct<'a>(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    v3: ref<uninit<int32>, borrowed, 'a, exclusive> = address (*v0).1
    call fill(v3): <'a>(ref<uninit<int32>, borrowed, 'a, exclusive>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// A call filling an uninitialized field through a later argument initializes that field.
#[test]
fn test_initialize_a_field_filled_through_a_later_call_argument() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

external function fill<'a>(int32, ref<uninit<int32>, borrowed, 'a, exclusive>): void

constructor construct<'a>(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    v3: ref<uninit<int32>, borrowed, 'a, exclusive> = address (*v0).1
    call fill(v1, v3): <'a>(int32, ref<uninit<int32>, borrowed, 'a, exclusive>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// Reject an address viewing the receiver storage as another type as invalid MIR.
#[test]
fn test_reject_an_address_viewing_the_receiver_as_another_type() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

external function fill<'a>(ref<uninit<int32>, borrowed, 'a, exclusive>): void

constructor construct<'a>(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>): void {
entry(v0: ref<uninit<Pair>, borrowed, 'a, exclusive>):
    v1: ref<uninit<int32>, borrowed, 'a, exclusive> = address (*v0)
    call fill(v1): <'a>(ref<uninit<int32>, borrowed, 'a, exclusive>) => void
    return
}
"#,
    );

    program.assert_invalid_mir(
        r#"
invalid MIR: an address of type 'ref<uninit<int32>, borrowed, 'a, exclusive>' over a place of type 'uninit<Pair>' in 'construct'
"#,
    );
}

/// Reject a load of a field past its struct as invalid MIR.
#[test]
fn test_reject_a_load_of_an_untyped_place() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): void {
entry(v0: ref<Box, borrowed, 'a, readonly>):
    v1: int32 = load (*v0).1
    return
}
"#,
    );

    program.assert_invalid_mir(
        r#"
invalid MIR: a memory operation on an untyped place in 'test'
"#,
    );
}
