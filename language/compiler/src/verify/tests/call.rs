use crate::tests::{TestProgram, TestSession};

/// Rest arguments initialize fresh storage across multiple spreads.
#[test]
fn test_pack_spread_arguments() {
    let session = TestSession::single(
        r#"
declare function take(...values: ^[int32]): void;

declare function change(values: int32[]): int32;

function forward(values: int32[]): void {
    take(0, ...values, change(values), ...[4, 5]);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A call result carries each argument path lifetime to the same result path.
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

/// A field of a call result keeps the lifetime of its own path.
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

/// Arguments of one lifetime satisfy an outlives bound between two parameters.
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

/// A frame borrow cannot outlive a caller borrow as an outlives bound requires.
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
    store l0, v2
    v3: ref<Box, borrowed, 'frame, mutable> = address l0
    v4: ref<int32, borrowed, 'frame, mutable> = address (*v3).0
    call callee(v4, v0): <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:20:5
   │
18 │     v3: ref<Box, borrowed, 'frame, mutable> = address l0
19 │     v4: ref<int32, borrowed, 'frame, mutable> = address (*v3).0
20 │ ·· <'a, 'c>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'c, mutable>) => void where 'a: 'c
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
21 │     return
22 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

/// A merged argument of two lifetimes breaks an outlives bound on either one.
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

join(v3: ref<int32, borrowed, 'a | 'b, readonly>):
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
11 │ join(v3: ref<int32, borrowed, 'a | 'b, readonly>):
12 │ ··'a, 'b>(ref<int32, borrowed, 'a, readonly>, ref<int32, borrowed, 'b, readonly>) => void where 'a: 'b
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
external function update<'a, 'b>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'b, mutable>): void

function test<'a>(v0: ref<int32, borrowed, 'a, mutable>): void {
entry(v0: ref<int32, borrowed, 'a, mutable>):
    call update(v0, v0): <'a, 'b>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'b, mutable>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// Mutable borrows of two distinct locals pass to one call.
#[test]
fn test_allow_distinct_mutable_arguments() {
    let mut program = TestProgram::mir(
        r#"
external function update<'a, 'b>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'b, mutable>): void

function test(): void {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, 'frame, mutable> = address l0
    v1: ref<int32, borrowed, 'frame, mutable> = address l1
    call update(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, mutable>, ref<int32, borrowed, 'b, mutable>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// Reject a readonly argument for a mutable parameter as invalid MIR.
#[test]
fn test_reject_writable_call_argument_from_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
external function update<'a>(ref<int32, borrowed, 'a, mutable>): void

function test<'a>(v0: ref<int32, borrowed, 'a, readonly>): void {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    call update(v0): <'a>(ref<int32, borrowed, 'a, mutable>) => void
    return
}
"#,
    );

    program.assert_invalid_mir(
        r#"
invalid MIR: a call argument of type 'ref<int32, borrowed, 'a, readonly>' where 'ref<int32, borrowed, 'a, mutable>' is expected in 'test'
"#,
    );
}

/// Allow a call writing Copy storage through a parameter during its reborrow.
#[test]
fn test_allow_a_copy_writing_call_through_a_parameter_during_its_reborrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function update<'a>(ref<Box, borrowed, 'a, mutable>): void

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): int32 {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    call update(v0): <'a>(ref<Box, borrowed, 'a, mutable>) => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"

"#,
    );
}

/// A tail call cannot pass a borrow of the frame it replaces.
#[test]
fn test_reject_frame_borrow_passed_by_tail_call() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function inspect<'a>(v0: ref<int32, borrowed, 'a, readonly>): void {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    return
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    tail.call inspect(v1): <'a>(ref<int32, borrowed, 'a, readonly>) => void
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:14:5
   │
12 │ entry(v0: ref<Box, unique, mutable>):
13 │     v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
14 │     tail.call inspect(v1): <'a>(ref<int32, borrowed, 'a, readonly>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// An aliasable argument verifies beside the aliasable receiver the call autorefs.
#[test]
fn test_allow_an_aliasable_argument_beside_its_autoref_receiver() {
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

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A fresh allocation's receiver cannot alias a borrowed argument the caller passed in.
#[test]
fn test_allow_a_mutable_receiver_of_a_fresh_allocation_beside_a_borrowed_argument() {
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

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// An unrelated callback can run before an owned allocation is published.
#[test]
fn test_allow_callback_before_publishing_owner() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function callback(): void
external function publish(ref<Box, unique, mutable>): void

function test(): int32 {
entry:
    v0: ref<Box, unique, mutable> = new.zeroed Box, local
    v1: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
    call callback(): () => void
    v2: int32 = load (*v1)
    call publish(v0): (ref<Box, unique, mutable>) => void
    return v2
}
"#,
    );

    program.assert_verified();
}

/// Passing a local borrow to one call permits an unrelated callback after that call returns.
#[test]
fn test_allow_callback_after_borrowed_argument() {
    let mut program = TestProgram::mir(
        r#"
external function inspect<'a>(ref<int32, borrowed, 'a, mutable>): void
external function callback(): void

function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, mutable> = address l0
    call inspect(v1): <'a>(ref<int32, borrowed, 'a, mutable>) => void
    call callback(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}

/// A callback cannot mutate a global while its exclusive borrow remains live.
#[test]
fn test_reject_callback_during_global_borrow() {
    let mut program = TestProgram::mir(
        r#"
global value: int32 = 0

external function callback(): void

function test(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, exclusive> = address @value
    call callback(): () => void
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(): int32 {
 7 │ entry:
 8 │     v0: ref<int32, borrowed, 'static, exclusive> = address @value
   │     ------------------------------------------------------------- borrow starts here
 9 │     call callback(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     v1: int32 = load (*v0)
11 │     return v1
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// Publishing an owner consumes it and invalidates its live field borrow.
#[test]
fn test_reject_publishing_borrowed_owner() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function publish(ref<Box, unique, mutable>): void

function test(): int32 {
entry:
    v0: ref<Box, unique, mutable> = new.zeroed Box, local
    v1: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
    call publish(v0): (ref<Box, unique, mutable>) => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:12:5
   │
 9 │ entry:
10 │     v0: ref<Box, unique, mutable> = new.zeroed Box, local
11 │     v1: ref<int32, borrowed, 'frame, mutable> = address (*v0).0
   │     ----------------------------------------------------------- borrow starts here
12 │     call publish(v0): (ref<Box, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
13 │     v2: int32 = load (*v1)
14 │     return v2
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// A reference stored by a call keeps its source borrowed while the destination is live.
#[test]
fn test_reject_write_while_call_stored_borrow_lives() {
    let mut program = TestProgram::mir(
        r#"
global initial: int32 = 0

external function save<'a, 'b>(ref<int32, borrowed, 'a, immutable>, ref<ref<int32, borrowed, 'a, immutable>, borrowed, 'b, mutable>): void

function test(v0: int32): int32 {
    local l0: int32
    local l1: ref<int32, borrowed, 'frame, immutable>

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'static, immutable> = address @initial
    store l1, v1
    v2: ref<int32, borrowed, 'frame, immutable> = address l0
    v3: ref<ref<int32, borrowed, 'frame, immutable>, borrowed, 'frame, mutable> = address l1
    call save(v2, v3): <'a, 'b>(ref<int32, borrowed, 'a, immutable>, ref<ref<int32, borrowed, 'a, immutable>, borrowed, 'b, mutable>) => void
    store l0, v0
    v4: ref<int32, borrowed, 'frame, immutable> = load l1
    v5: int32 = load (*v4)
    return v5
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:17:5
   │
12 │     v1: ref<int32, borrowed, 'static, immutable> = address @initial
13 │     store l1, v1
14 │     v2: ref<int32, borrowed, 'frame, immutable> = address l0
   │     -------------------------------------------------------- borrow starts here
15 │     v3: ref<ref<int32, borrowed, 'frame, immutable>, borrowed, 'frame, mutable> = address l1
16 │     call save(v2, v3): <'a, 'b>(ref<int32, borrowed, 'a, immutable>, ref<ref<int32, borrowed, 'a, im··
17 │     store l0, v0
   │     ^^^^^^^^^^^^
18 │     v4: ref<int32, borrowed, 'frame, immutable> = load l1
19 │     v5: int32 = load (*v4)
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// Overwriting a local destination ends the borrow that a previous call stored there.
#[test]
fn test_release_call_stored_borrow_after_overwrite() {
    let mut program = TestProgram::mir(
        r#"
global initial: int32 = 0

external function save<'a, 'b>(ref<int32, borrowed, 'a, readonly>, ref<ref<int32, borrowed, 'a, readonly>, borrowed, 'b, mutable>): void

function test(v0: int32): int32 {
    local l0: int32
    local l1: ref<int32, borrowed, 'frame, readonly>

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'static, readonly> = address @initial
    store l1, v1
    v2: ref<int32, borrowed, 'frame, readonly> = address l0
    v3: ref<ref<int32, borrowed, 'frame, readonly>, borrowed, 'frame, mutable> = address l1
    call save(v2, v3): <'a, 'b>(ref<int32, borrowed, 'a, readonly>, ref<ref<int32, borrowed, 'a, readonly>, borrowed, 'b, mutable>) => void
    store l1, v1
    store l0, v0
    v4: ref<int32, borrowed, 'frame, readonly> = load l1
    v5: int32 = load (*v4)
    return v5
}
"#,
    );

    program.assert_verified();
}

/// Call a base constructor on the receiver a derived constructor borrows.
#[test]
fn test_call_a_base_constructor_from_a_derived_constructor() {
    let session = TestSession::single(
        r#"
class Base {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}

class Derived extends Base {
    constructor(name: string) {
        super(name);
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// Pass a view derived from an immutable parameter beside a mutable borrow parameter.
#[test]
fn test_pass_an_immutable_view_beside_a_mutable_borrow_of_another_parameter() {
    let session = TestSession::single(
        r#"
struct Holder {
    value: int32;

    view(&readonly this): &readonly int32 {
        return &readonly this.value;
    }
}

struct Sink {
    count: int32;

    add(&exclusive this, value: &readonly int32): void {
        this.count += *value;
    }
}

function feed(source: &immutable Holder, sink: &exclusive Sink): void {
    sink.add(source.view());
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A frame borrow passed to a dynamic interface method has the method's parameter type.
#[test]
fn test_pass_a_frame_borrow_to_a_dynamic_interface_method() {
    let session = TestSession::single(
        r#"
interface Sink {
    emit(entry: &readonly int32): void;
}

export function run(sink: Dynamic<Sink>): void {
    const entry: int32 = 1;
    sink.emit(&readonly entry);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@nocopy
type test.main.Sink { }

function test.main.run(v0: dynamic<test.main.Sink, managed, mutable, local>): void {
    local l0: dynamic<test.main.Sink, managed, mutable, local>
    local l1: int32

entry(v0: dynamic<test.main.Sink, managed, mutable, local>):
    store l0, v0
    v1: int32 = 1
    store l1, v1
    v2: dynamic<test.main.Sink, managed, mutable, local> = load l0
    v3: ref<int32, borrowed, 'frame, readonly> = address l1
    call.dynamic v2, test.main.Sink, 0(v3): (ref<int32, borrowed, 'frame, readonly>) => void
    return
}

/// @layout.struct name=test.main.Sink size=0 align=1
/// @layout.struct name=type@6 size=0 align=1

/// @dispatch.shape constraint=type@0 function=emit
"#,
    );
    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
}

/// An inherited method call passes the derived handle at the base receiver type.
#[test]
fn test_call_an_inherited_method_on_a_derived_class_handle() {
    let session = TestSession::single(
        r#"
class Base<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(this): T {
        this.value
    }
}

class Derived<T: Copy> extends Base<T> {
    constructor(value: T) {
        super(value);
    }
}

const derived: Derived<int32> = new Derived<int32>(1);

export function run(): int32 {
    derived.get()
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
@nocopy
type test.main.Derived<T: Copy> {
    value: T;
}

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
type test.main.Base<T: Copy> {
    value: T;
}

global test.main.derived: ref<test.main.Derived<int32>, managed, mutable, local> = zeroinit

function test.main.run(): int32 {
entry:
    v0: ref<test.main.Derived<int32>, managed, mutable, local> = load @test.main.derived
    v1: ref<test.main.Base<int32>, managed, mutable, local> = cast.bit v0 -> ref<test.main.Base<int32>, managed, mutable, local>
    v2: int32 = call test.main.Base.get<int32>(v1): (ref<test.main.Base<int32>, managed, mutable, local>) => int32
    return v2
}

constructor test.main.Base.constructor<T: Copy>(v0: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>, v1: T):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable> = load l1
    v3: T = load l0
    store (*v2).0, v3
    return
}

function test.main.Base.get<T: Copy>(v0: ref<test.main.Base<T>, managed, mutable, local>): T {
    local l0: ref<test.main.Base<T>, managed, mutable, local>

entry(v0: ref<test.main.Base<T>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Base<T>, managed, mutable, local> = load l0
    v2: T = load (*v1).0
    return v2
}

constructor test.main.Derived.constructor<T: Copy>(v0: ref<uninit<test.main.Derived<T>>, borrowed, 'managed, mutable>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Derived<T>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Derived<T>>, borrowed, 'managed, mutable>, v1: T):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Derived<T>>, borrowed, 'managed, mutable> = address (*l1)
    v3: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>
    v4: T = load l0
    call test.main.Base.constructor<T>(v3, v4): (ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>, T) => void
    v5: void = zeroed
    return
}

shared function test.main.Base.get<int32>(v0: ref<test.main.Base<int32>, managed, mutable, local>): int32;

export function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: ref<test.main.Derived<int32>, managed, mutable, local> = new.zeroed test.main.Derived<int32>, local
    v2: ref<uninit<test.main.Derived<int32>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Derived<int32>>, borrowed, 'managed, mutable>
    call test.main.Derived.constructor<int32>(v2, v0): (ref<uninit<test.main.Derived<int32>>, borrowed, 'managed, mutable>, int32) => void
    store @test.main.derived, v1
    return
}

shared constructor test.main.Derived.constructor<int32>(v0: ref<uninit<test.main.Derived<int32>>, borrowed, 'managed, mutable>, v1: int32): void;

/// @layout.struct name=test.main.Derived<int32> size=4 align=4
/// @layout.field owner=test.main.Derived<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.main.Base<int32> size=4 align=4
/// @layout.field owner=test.main.Base<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@40 size=4 align=4
/// @layout.field owner=type@40 index=0 name=value offset=0 size=4 align=4

/// @dispatch.shape constraint=type@3 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@6 function=clone function=cloneFrom
"#);
    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
}

/// Reject a tail call argument of another type than its parameter as invalid MIR.
#[test]
fn test_reject_a_tail_call_argument_of_another_type() {
    let mut program = TestProgram::mir(
        r#"
function inspect(v0: int32): void {
entry(v0: int32):
    return
}

function test(v0: int64): void {
entry(v0: int64):
    tail.call inspect(v0): (int32) => void
}
"#,
    );

    program.assert_invalid_mir(
        r#"
invalid MIR: a call argument of type 'int64' where 'int32' is expected in 'test'
"#,
    );
}

/// Allow a call writing a Copy local through a merged reference under a readonly loan.
#[test]
fn test_allow_a_call_writing_a_copy_local_through_a_merged_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function write<'a>(ref<int32, borrowed, 'a, mutable>): void

function test(v0: ref<Box, managed, mutable, local>, v1: int32, v2: boolean): int32 {
    local l0: int32

entry(v0: ref<Box, managed, mutable, local>, v1: int32, v2: boolean):
    store l0, v1
    v3: ref<int32, borrowed, 'frame, mutable> = address l0
    v4: ref<int32, borrowed, 'managed, mutable> = address (*v0).0
    v5: ref<int32, borrowed, 'frame | 'managed, mutable> = select v2, v3, v4
    v6: ref<int32, borrowed, 'frame, readonly> = address l0
    call write(v5): <'a>(ref<int32, borrowed, 'a, mutable>) => void
    v7: int32 = load (*v6)
    return v7
}
"#,
    );

    program.assert_verify_errors(
        r#"

"#,
    );
}

/// One argument merging two exclusive borrows of a local passes to one exclusive parameter.
#[test]
fn test_allow_a_merged_exclusive_argument() {
    let mut program = TestProgram::mir(
        r#"
external function write<'a>(ref<int32, borrowed, 'a, exclusive>): void

function test(v0: int32, v1: boolean): void {
    local l0: int32

entry(v0: int32, v1: boolean):
    store l0, v0
    branch v1 => left | right

left:
    v2: ref<int32, borrowed, 'frame, exclusive> = address l0
    jump join(v2)

right:
    v3: ref<int32, borrowed, 'frame, exclusive> = address l0
    jump join(v3)

join(v4: ref<int32, borrowed, 'frame, exclusive>):
    call write(v4): <'a>(ref<int32, borrowed, 'a, exclusive>) => void
    return
}
"#,
    );

    program.assert_verified();
}
