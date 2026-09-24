use crate::tests::TestSession;

/// Lower a module constant to a global read through its address.
#[test]
fn test_lower_module_constants_to_globals() {
    let session = TestSession::single(
        r#"
newtype Flags = uint32

const READ: Flags = Flags(4);

function pick(): Flags {
    return READ;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
type test.main.Flags = newtype<uint32>;

function test.main.pick(): test.main.Flags {
entry:
    v0: test.main.Flags = load @test.main.READ
    return v0
}
"#,
    );
}

/// Store a constant with a runtime initializer from the module initializer.
#[test]
fn test_store_a_runtime_binding_from_the_module_initializer() {
    let session = TestSession::single(
        r#"
function seed(): int32 {
    return 41;
}

const start = seed() + 1;

function run(): int32 {
    return start;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.seed",
        r#"
function test.main.seed(): int32 {
entry:
    v0: int32 = 41
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
function test.main.run(): int32 {
entry:
    v0: int32 = load @test.main.start
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.@init",
        r#"
export function test.main.@init(): void {
entry:
    v0: int32 = call test.main.seed(): () => int32
    v1: int32 = 1
    v2: int32 = add v0, v1
    store @test.main.start, v2
    return
}
"#,
    );
}

/// Store a struct constant from the module initializer until constant aggregates land.
#[test]
fn test_store_a_struct_constant_from_the_module_initializer() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const ORIGIN = Point { x: 1 };

function pick(): Point {
    return ORIGIN;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
type test.main.Point {
    x: int32;
}

function test.main.pick(): test.main.Point {
entry:
    v0: test.main.Point = load @test.main.ORIGIN
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.@init",
        r#"
type test.main.Point {
    x: int32;
}

export function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: test.main.Point = aggregate (v0)
    store @test.main.ORIGIN, v1
    return
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// Pair a generic extension's drop hook with each nominal specialization.
#[test]
fn test_pair_generic_extension_drop_hook() {
    let session = TestSession::single(
        r#"
import { Drop, drop } from "destack:memory";

struct Guard<T> {
    value: T;
}

extension<T> of Guard<T> implements Drop {
    drop(&this): void {}
}

function consume(guard: Guard<int32>): void {
    drop(guard);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.consume",
        r#"
@nocopy
type test.main.Guard<T> {
    value: T;
}

function test.main.consume(v0: test.main.Guard<int32>): void {
    local l0: test.main.Guard<int32>

entry(v0: test.main.Guard<int32>):
    store l0, v0
    v1: test.main.Guard<int32> = load l0
    drop v1
    return
}

/// @layout.struct name=test.main.Guard<int32> size=4 align=4
/// @layout.field owner=test.main.Guard<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Guard.Drop.drop",
        r#"
@nocopy
type test.main.Guard<T> {
    value: T;
}

function test.main.Guard.Drop.drop<T, 'a>(v0: ref<test.main.Guard<T>, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.Guard<T>, borrowed, 'a, mutable>

entry(v0: ref<test.main.Guard<T>, borrowed, 'a, mutable>):
    store l0, v0
    return
}
"#,
    );
}

/// Extension members take their target root and, for conformance members, their interface.
#[test]
fn test_name_extension_members_by_target_root_and_interface() {
    let session = TestSession::single(
        r#"
newtype interface Greet {
    greet(&readonly this): int32;
}

struct Cell {
    value: int32;
}

export extension of Cell implements Greet {
    greet(&readonly this): int32 {
        return this.value;
    }

    peek(&readonly this): int32 {
        return this.value;
    }
}

export extension<T: Greet> of T {
    twice(&readonly this): int32 {
        return this.greet() + this.greet();
    }
}
"#,
    );
    session.assert_mir_lowered(
        "main.ds",
        r#"
type test.main.Cell {
    value: int32;
}

@nocopy
type test.main.Greet { }

function test.main.Cell.Greet.greet<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Cell, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

function test.main.Cell.peek<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Cell, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

function test.main.Greet.twice<T: test.main.Greet, 'a>(v0: ref<?T, borrowed, 'a, readonly>): int32 {
    local l0: ref<?T, borrowed, 'a, readonly>

entry(v0: ref<?T, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<?T, borrowed, 'a, readonly> = load l0
    v2: int32 = call.witness T, test.main.Greet, test.main.Greet.greet(v1): (ref<?T, borrowed, 'a, readonly>) => int32
    v3: ref<?T, borrowed, 'a, readonly> = load l0
    v4: int32 = call.witness T, test.main.Greet, test.main.Greet.greet(v3): (ref<?T, borrowed, 'a, readonly>) => int32
    v5: int32 = add v2, v4
    return v5
}

external function test.main.Greet.greet<this: test.main.Greet, 'a>(ref<?this, borrowed, 'a, readonly>): int32

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@2 size=4 align=4
/// @layout.field owner=type@2 index=0 name=value offset=0 size=4 align=4

/// @dispatch.shape constraint=type@7 function=greet
"#,
    );
}
