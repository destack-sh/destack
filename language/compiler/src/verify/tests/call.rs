use crate::tests::{TestProgram, TestSession};

#[test]
fn test_propagate_call_result_aggregate_path_origin() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly, local>;
    right: ref<int32, borrowed, 'B, readonly, local>;
}

function callee<'a, 'b>(v0: Pair<'a & local, 'b & local>): Pair<'a & local, 'b & local> {
b0(v0: Pair<'a & local, 'b & local>):
    return v0
}

function caller<'a, 'b>(v0: Pair<'a & local, 'b & local>): ref<int32, borrowed, 'b, readonly, local> {
b1(v0: Pair<'a & local, 'b & local>):
    v1: Pair<'a & local, 'b & local> = call callee(v0): (Pair<'a & local, 'b & local>) => Pair<'a & local, 'b & local>
    v2: ref<int32, borrowed, 'b, readonly, local> = field.get v1, 1
    return v2
}"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_call_result_aggregate_wrong_path_origin() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly, local>;
    right: ref<int32, borrowed, 'B, readonly, local>;
}

function callee<'a, 'b>(v0: Pair<'a & local, 'b & local>): Pair<'a & local, 'b & local> {
b0(v0: Pair<'a & local, 'b & local>):
    return v0
}

function caller<'a, 'b>(v0: Pair<'a & local, 'b & local>): ref<int32, borrowed, 'a, readonly, local> {
b1(v0: Pair<'a & local, 'b & local>):
    v1: Pair<'a & local, 'b & local> = call callee(v0): (Pair<'a & local, 'b & local>) => Pair<'a & local, 'b & local>
    v2: ref<int32, borrowed, 'b, readonly, local> = field.get v1, 1
    return v2
}"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:16:5
   │
14 │     v1: Pair<'a & local, 'b & local> = call callee(v0): (Pair<'a & local, 'b & local>) => Pair<'a & ··
15 │     v2: ref<int32, borrowed, 'b, readonly, local> = field.get v1, 1
16 │     return v2
   │     ^^^^^^^^^
17 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

#[test]
fn test_allow_call_arguments_satisfying_outlives_bounds() {
    let mut program = TestProgram::mir(
        r#"
function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'c, mutable, local>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'c, mutable, local>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable, local>): void {
entry(v0: ref<int32, borrowed, 'L, mutable, local>):
    call callee(v0, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'c, mutable, local>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_call_arguments_violating_outlives_bounds() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'c, mutable, local>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'c, mutable, local>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable, local>): void {
    local l0: Box

entry(v0: ref<int32, borrowed, 'L, mutable, local>):
    v1: int32 = 0
    v2: Box = aggregate (v1)
    local.set l0, v2
    v3: ref<Box, borrowed, 'frame, mutable, frame> = local.address l0
    v4: ref<int32, borrowed, 'frame, mutable, local> = field.address v3, 0
    call callee(v4, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'c, mutable, local>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:20:5
   │
18 │     v3: ref<Box, borrowed, 'frame, mutable, frame> = local.address l0
19 │     v4: ref<int32, borrowed, 'frame, mutable, local> = field.address v3, 0
20 │ ··int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'c, mutable, local>) => void where 'a: 'c
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
21 │     return
22 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

#[test]
fn test_reject_call_bound_against_merged_shorter_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function callee<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'b, readonly, local>): void where 'a: 'b {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'b, readonly, local>):
    return
}

function caller<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'b, readonly, local>, v2: boolean): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'b, readonly, local>, v2: boolean):
    branch v2 => join(v0) | join(v1)

join(v3: ref<int32, borrowed, 'a | 'b, readonly, local>):
    call callee(v0, v3): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => void where 'a: 'b
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:12:5
   │
10 │
11 │ join(v3: ref<int32, borrowed, 'a | 'b, readonly, local>):
12 │ ··t32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => void where 'a: 'b
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
13 │     return
14 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

/// Mutable arguments reborrowed from one parameter may alias, the storage behind it unknown.
#[test]
fn test_allow_aliased_mutable_arguments_through_a_parameter() {
    let mut program = TestProgram::mir(
        r#"
external function update<'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>): void

function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): void {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    call update(v0, v0): <'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>) => void
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_distinct_mutable_arguments() {
    let mut program = TestProgram::mir(
        r#"
external function update<'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>): void

function test(): void {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l1
    call update(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>) => void
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_writable_call_argument_from_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
external function update<'a>(ref<int32, borrowed, 'a, mutable, local>): void

function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    call update(v0): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-through-readonly-reference]: cannot create a writable borrow through a readonly reference
 ──▶ <test.dsm>:6:5
  │
4 │ function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): void {
5 │ entry(v0: ref<int32, borrowed, 'a, readonly, local>):
6 │     call update(v0): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     return
8 │ }
  │

for more information about an error, run `destack explain borrow-through-readonly-reference`
"#,
    );
}

/// A parameter may be passed on while a field reborrowed through it is live.
#[test]
fn test_allow_a_call_through_a_parameter_during_its_reborrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function update<'a>(ref<Box, borrowed, 'a, mutable, local>): void

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = field.address v0, 0
    call update(v0): <'a>(ref<Box, borrowed, 'a, mutable, local>) => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_frame_borrow_passed_by_tail_call() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function inspect<'a>(v0: ref<int32, borrowed, 'a, readonly, frame>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, frame>):
    return
}

function test(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    v1: ref<int32, borrowed, 'frame, readonly, frame> = field.address v0, 0
    tail.call inspect(v1): <'a>(ref<int32, borrowed, 'a, readonly, frame>) => void
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:14:5
   │
12 │ entry(v0: ref<Box, unique, mutable, local>):
13 │     v1: ref<int32, borrowed, 'frame, readonly, frame> = field.address v0, 0
14 │     tail.call inspect(v1): <'a>(ref<int32, borrowed, 'a, readonly, frame>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
15 │ }
16 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

/// A readonly argument borrow ends before the receiver's borrow begins.
#[test]
fn test_allow_a_readonly_argument_borrow_ending_before_the_receiver_borrow() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32;

    add(&this, amount: int32): void {
        this.count += amount;
    }

    get(&readonly this): int32 {
        return this.count;
    }
}

export function drive(): int32 {
    let counter = Counter { count: 1 };
    counter.add(counter.get());
    return counter.get();
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}

/// An exclusive argument cannot alias the receiver the call borrows exclusively.
#[test]
fn test_reject_an_mutable_argument_beside_its_autoref_receiver() {
    let session = TestSession::single(
        r#"
struct Foo {
    value: int32;

    method(&this, other: &Foo): void {}
}

export function multiMut(): void {
    let foo = Foo { value: 0 };
    foo.method(&foo);
}

export function accessDuringReservation(): void {
    let i = 0;
    const p = &i;
    const j = i;
    *p += 1;
    const k = i;
    *p += 1;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=10 column=5 span="foo.method(&foo)" line_source="foo.method(&foo);"
/// @diagnostic.related line=10 column=16 span="&foo" line_source="foo.method(&foo);" message="borrow starts here"
/// @diagnostic.error id=mutable-argument-alias message="mutable call arguments may refer to the same owned storage"
/// @diagnostic.label line=10 column=5 span="foo.method(&foo)" line_source="foo.method(&foo);"
/// @diagnostic.error id=use-of-mutably-borrowed-place message="cannot use mutably borrowed place"
/// @diagnostic.label line=16 column=15 span="i" line_source="const j = i;"
/// @diagnostic.related line=15 column=15 span="&i" line_source="const p = &i;" message="borrow starts here"
"#);
}

/// A fresh allocation's receiver cannot alias a borrowed argument the caller passed in.
#[test]
fn test_allow_an_mutable_receiver_of_a_fresh_allocation_beside_a_borrowed_argument() {
    let session = TestSession::single(
        r#"
class Span {
    name: string;

    constructor(name: &readonly string) {
        this.name = name.slice();
    }
}

function start(name: &readonly string): Span {
    return new Span(name);
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#""#);
}
