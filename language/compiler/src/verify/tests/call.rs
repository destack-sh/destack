use crate::tests::{TestProgram, TestSession};

#[test]
fn test_propagate_call_result_aggregate_path_origin() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function callee<'a, 'b>(v0: Pair<'a, 'b>): Pair<'a, 'b> {
b0(v0: Pair<'a, 'b>):
    return v0
}

function caller<'a, 'b>(v0: Pair<'a, 'b>): ref<int32, borrowed, 'b, readonly> {
b1(v0: Pair<'a, 'b>):
    v1: Pair<'a, 'b> = call callee(v0): (Pair<'a, 'b>) => Pair<'a, 'b>
    v2: ref<int32, borrowed, 'b, readonly> = field.get v1, 1
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
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function callee<'a, 'b>(v0: Pair<'a, 'b>): Pair<'a, 'b> {
b0(v0: Pair<'a, 'b>):
    return v0
}

function caller<'a, 'b>(v0: Pair<'a, 'b>): ref<int32, borrowed, 'a, readonly> {
b1(v0: Pair<'a, 'b>):
    v1: Pair<'a, 'b> = call callee(v0): (Pair<'a, 'b>) => Pair<'a, 'b>
    v2: ref<int32, borrowed, 'b, readonly> = field.get v1, 1
    return v2
}"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:16:5
   │
14 │     v1: Pair<'a, 'b> = call callee(v0): (Pair<'a, 'b>) => Pair<'a, 'b>
15 │     v2: ref<int32, borrowed, 'b, readonly> = field.get v1, 1
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
function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable>): void {
entry(v0: ref<int32, borrowed, 'L, mutable>):
    call callee(v0, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
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

function callee<'a, 'c>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>): void where 'a: 'c {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'c, mutable>):
    return
}

function caller<'L>(v0: ref<int32, borrowed, 'L, mutable>): void {
    local l0: Box

entry(v0: ref<int32, borrowed, 'L, mutable>):
    v1: int32 = 0
    v2: Box = aggregate (v1)
    local.set l0, v2
    v3: ref<Box, borrowed, mutable, frame> = local.address l0
    v4: ref<int32, borrowed, mutable> = field.address v3, 0
    call callee(v4, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:20:5
   │
18 │     v3: ref<Box, borrowed, mutable, frame> = local.address l0
19 │     v4: ref<int32, borrowed, mutable> = field.address v3, 0
20 │ ·· <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
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
function callee<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>): void where 'a: 'b {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>):
    return
}

function caller<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean): void {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean):
    branch v2 => join(v0) | join(v1)

join(v3: ref<int32, borrowed, readonly>):
    call callee(v0, v3): <'a, 'b>(ref<int32, borrowed, 'a, readonly>, ref<int32, borrowed, 'b, readonly>) => void where 'a: 'b
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:12:5
   │
10 │
11 │ join(v3: ref<int32, borrowed, readonly>):
12 │ ··'a, 'b>(ref<int32, borrowed, 'a, readonly>, ref<int32, borrowed, 'b, readonly>) => void where 'a: 'b
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
13 │     return
14 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

#[test]
fn test_reject_aliased_exclusive_arguments() {
    let mut program = TestProgram::mir(
        r#"
external function update(ref<int32, borrowed, exclusive>, ref<int32, borrowed, exclusive>): void

function test(v0: ref<int32, borrowed, mutable>): void {
entry(v0: ref<int32, borrowed, mutable>):
    call update(v0, v0): (ref<int32, borrowed, exclusive>, ref<int32, borrowed, exclusive>) => void
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[exclusive-argument-alias]: exclusive call arguments may refer to the same storage
 ──▶ <test.dsm>:6:5
  │
4 │ function test(v0: ref<int32, borrowed, mutable>): void {
5 │ entry(v0: ref<int32, borrowed, mutable>):
6 │     call update(v0, v0): (ref<int32, borrowed, exclusive>, ref<int32, borrowed, exclusive>) => void
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     return
8 │ }
  │

for more information about an error, run `destack explain exclusive-argument-alias`
"#,
    );
}

#[test]
fn test_allow_distinct_exclusive_arguments() {
    let mut program = TestProgram::mir(
        r#"
external function update(ref<int32, borrowed, exclusive>, ref<int32, borrowed, exclusive>): void

function test(): void {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    call update(v0, v1): (ref<int32, borrowed, exclusive>, ref<int32, borrowed, exclusive>) => void
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
external function update(ref<int32, borrowed, mutable>): void

function test(v0: ref<int32, borrowed, readonly>): void {
entry(v0: ref<int32, borrowed, readonly>):
    call update(v0): (ref<int32, borrowed, mutable>) => void
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-through-readonly-reference]: cannot create a writable borrow through a readonly reference
 ──▶ <test.dsm>:6:5
  │
4 │ function test(v0: ref<int32, borrowed, readonly>): void {
5 │ entry(v0: ref<int32, borrowed, readonly>):
6 │     call update(v0): (ref<int32, borrowed, mutable>) => void
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │     return
8 │ }
  │

for more information about an error, run `destack explain borrow-through-readonly-reference`
"#,
    );
}

#[test]
fn test_reject_call_through_root_during_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function update(ref<Box, borrowed, mutable>): void

function test(v0: ref<Box, borrowed, mutable>): int32 {
entry(v0: ref<Box, borrowed, mutable>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    call update(v0): (ref<Box, borrowed, mutable>) => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:11:5
   │
 8 │ function test(v0: ref<Box, borrowed, mutable>): int32 {
 9 │ entry(v0: ref<Box, borrowed, mutable>):
10 │     v1: ref<int32, borrowed, exclusive> = field.address v0, 0
   │     --------------------------------------------------------- borrow starts here
11 │     call update(v0): (ref<Box, borrowed, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     v2: int32 = load v1
13 │     return v2
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

#[test]
fn test_reject_frame_borrow_passed_by_tail_call() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function inspect(v0: ref<int32, borrowed, frame, readonly>): void {
entry(v0: ref<int32, borrowed, frame, readonly>):
    return
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, frame, readonly> = field.address v0, 0
    tail.call inspect(v1): (ref<int32, borrowed, frame, readonly>) => void
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:14:5
   │
12 │ entry(v0: ref<Box, unique, mutable>):
13 │     v1: ref<int32, borrowed, frame, readonly> = field.address v0, 0
14 │     tail.call inspect(v1): (ref<int32, borrowed, frame, readonly>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
15 │ }
16 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

/// A readonly argument borrow ends before the receiver's exclusive borrow begins.
#[test]
fn test_allow_a_readonly_argument_borrow_ending_before_the_receiver_borrow() {
    let session = TestSession::single(
        r#"
struct Counter {
    count: int32;

    add(&exclusive this, amount: int32): void {
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
fn test_reject_an_exclusive_argument_beside_its_autoref_receiver() {
    let session = TestSession::single(
        r#"
struct Foo {
    value: int32;

    method(&exclusive this, other: &exclusive Foo): void {}
}

export function multiMut(): void {
    let foo = Foo { value: 0 };
    foo.method(&exclusive foo);
}

export function accessDuringReservation(): void {
    let i = 0;
    const p = &exclusive i;
    const j = i;
    *p += 1;
    const k = i;
    *p += 1;
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=borrow-conflict message="borrow conflicts with active borrow"
/// @diagnostic.label line=10 column=5 span="foo.method(&exclusive foo)" line_source="foo.method(&exclusive foo);"
/// @diagnostic.related line=10 column=16 span="&exclusive foo" line_source="foo.method(&exclusive foo);" message="borrow starts here"
/// @diagnostic.error id=exclusive-argument-alias message="exclusive call arguments may refer to the same storage"
/// @diagnostic.label line=10 column=5 span="foo.method(&exclusive foo)" line_source="foo.method(&exclusive foo);"
/// @diagnostic.error id=use-of-exclusively-borrowed-place message="cannot use exclusively borrowed place"
/// @diagnostic.label line=16 column=15 span="i" line_source="const j = i;"
/// @diagnostic.related line=15 column=15 span="&exclusive i" line_source="const p = &exclusive i;" message="borrow starts here"
"#);
}

/// A fresh allocation's exclusive receiver cannot alias a borrowed argument the caller passed in.
#[test]
fn test_allow_an_exclusive_receiver_of_a_fresh_allocation_beside_a_borrowed_argument() {
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
