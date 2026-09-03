use crate::tests::{TestProgram, TestSession};

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
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    branch v1 => b1(v2) | b2

b1(v3: ref<int32, borrowed, mutable>):
    call consume(v0): (ref<Box, unique, mutable>) => void
    v4: int32 = load v3
    return

b2:
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:17:5
   │
11 │ function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
12 │ entry(v0: ref<Box, unique, mutable>, v1: boolean):
13 │     v2: ref<int32, borrowed, mutable> = field.address v0, 0
   │     ------------------------------------------------------- borrow starts here
14 │     branch v1 => b1(v2) | b2
15 │
16 │ b1(v3: ref<int32, borrowed, mutable>):
17 │     call consume(v0): (ref<Box, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
18 │     v4: int32 = load v3
19 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

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
    v1: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:10:5
   │
 7 │ function test(v0: ref<int32, unique, mutable>): void {
 8 │ entry(v0: ref<int32, unique, mutable>):
 9 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ------------------------------------------------------- value moved here
10 │     v1: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

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
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(v0: ref<int32, unique, mutable>): void {
 7 │ entry(v0: ref<int32, unique, mutable>):
 8 │     v1: Box = aggregate (v0)
   │     ------------------------ value moved here
 9 │     v2: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

#[test]
fn test_reject_use_after_new_complete() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): void {
entry:
    v0: uninit<ref<Box, managed, mutable>> = new.uninit Box
    v1: ref<Box, managed, mutable> = new.complete v0
    v2: ref<Box, managed, mutable> = new.complete v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:10:5
   │
 7 │ entry:
 8 │     v0: uninit<ref<Box, managed, mutable>> = new.uninit Box
 9 │     v1: ref<Box, managed, mutable> = new.complete v0
   │     ------------------------------------------------ value moved here
10 │     v2: ref<Box, managed, mutable> = new.complete v0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

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
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.dsm>:19:5
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
19 │     v2: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
20 │     return
21 │ }
   │

for more information about an error, run `destack explain maybe-use-after-move`
"#,
    );
}

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

    program.assert_verify_errors(r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.dsm>:13:5
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

for more information about an error, run `destack explain maybe-use-after-move`
"#);
}

#[test]
fn test_reject_constructor_return_with_uninitialized_field() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int32;
}

function construct(v0: ref<uninit<Pair>, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Pair>, borrowed, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[field-left-uninitialized]: constructor returns before initializing field 1
  ──▶ <test.dsm>:11:5
   │
 9 │     v2: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
10 │     store v2, v1
11 │     return
   │     ^^^^^^
12 │ }
13 │
   │

for more information about an error, run `destack explain field-left-uninitialized`
"#,
    );
}

#[test]
fn test_reject_constructor_field_initialized_twice() {
    let mut program = TestProgram::mir(
        r#"
type Single {
    value: int32;
}

function construct(v0: ref<uninit<Single>, borrowed, exclusive>, v1: int32): void {
entry(v0: ref<uninit<Single>, borrowed, exclusive>, v1: int32):
    v2: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    v3: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
    store v3, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[field-initialized-twice]: constructor initializes field 0 twice
  ──▶ <test.dsm>:11:5
   │
 9 │     store v2, v1
10 │     v3: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
11 │     store v3, v1
   │     ^^^^^^^^^^^^
12 │     return
13 │ }
   │

for more information about an error, run `destack explain field-initialized-twice`
"#,
    );
}

/// A binding declared inside a loop body starts each iteration uninitialized.
#[test]
fn test_reject_a_use_of_a_binding_uninitialized_in_this_iteration() {
    let session = TestSession::single(
        r#"
export function ok(): void {
    loop {
        const x = 1;
    }
}

export function fail(): void {
    loop {
        let x: int32;
        const y = x + 1;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=11 column=19 span="x" line_source="const y = x + 1;"
/// @diagnostic.related line=10 column=13 span="x" line_source="let x: int32;" message="declared here"
"#);
}

/// A reinitialized binding moves again only once.
#[test]
fn test_reject_a_use_after_the_second_move_of_a_reinitialized_binding() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&exclusive this): void {}
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

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=15 column=13 span="x" line_source="consume(x);"
/// @diagnostic.related line=14 column=13 span="x" line_source="consume(x);" message="value moved here"
"#);
}

/// A binding read before every path assigns it is uninitialized.
#[test]
fn test_reject_uses_of_uninitialized_bindings() {
    let session = TestSession::single(
        r#"
function foo(x: int32): void {}

export function uninit(): void {
    let x: int32;
    foo(x);
}

export function ifNoElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    }
    foo(x);
}

export function ifWithElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    } else {
        x = 20;
    }
    foo(x);
}

export function whileCond(): void {
    let x: boolean;
    while (x) {}
}
"#,
    );

    session.assert_mir_verified_diagnostics("main.ds", r#"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=6 column=9 span="x" line_source="foo(x);"
/// @diagnostic.related line=5 column=9 span="x" line_source="let x: int32;" message="declared here"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=14 column=9 span="x" line_source="foo(x);"
/// @diagnostic.related line=10 column=9 span="x" line_source="let x: int32;" message="declared here"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=29 column=12 span="x" line_source="while (x) {}"
/// @diagnostic.related line=28 column=9 span="x" line_source="let x: boolean;" message="declared here"
"#);
}

/// A callee initializes the storage its uninitialized argument addresses.
#[test]
fn test_allow_reading_storage_a_callee_initialized() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function build(v0: ref<uninit<Box>, borrowed, exclusive>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<uninit<Box>, borrowed, exclusive>, v1: ref<int32, unique, mutable>):
    v2: ref<uninit<ref<int32, unique, mutable>>, borrowed, exclusive> = field.project v0, 0
    store v2, v1
    return
}

function test(v0: ref<int32, unique, mutable>): Box {
    local l0: Box

entry(v0: ref<int32, unique, mutable>):
    v1: ref<Box, borrowed, exclusive, frame> = local.address l0
    v2: ref<uninit<Box>, borrowed, exclusive, frame> = cast.bit v1 -> ref<uninit<Box>, borrowed, exclusive, frame>
    call build(v2, v0): (ref<uninit<Box>, borrowed, exclusive>, ref<int32, unique, mutable>) => void
    v3: Box = local.get l0
    return v3
}
"#,
    );

    program.assert_verified();
}
