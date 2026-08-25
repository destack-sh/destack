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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Flags = newtype<uint32>;

constant test.main.READ: Flags = {4uint32}

function test.main.pick(): Flags {
entry:
    v0: ref<Flags, borrowed, readonly, constant> = global.address test.main.READ
    v1: Flags = load v0
    return v1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
global test.main.start: int32 = zeroinit

function test.main.seed(): int32 {
entry:
    v0: int32 = 41
    return v0
}

function test.main.run(): int32 {
entry:
    v0: ref<int32, borrowed, readonly, static> = global.address test.main.start
    v1: int32 = load v0
    return v1
}

function test.main.@init(): void {
entry:
    v0: int32 = call test.main.seed(): () => int32
    v1: int32 = 1
    v2: int32 = add v0, v1
    v3: ref<int32, borrowed, mutable, static> = global.address test.main.start
    store v3, v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
}

global test.main.ORIGIN: Point = zeroinit

function test.main.pick(): Point {
entry:
    v0: ref<Point, borrowed, readonly, static> = global.address test.main.ORIGIN
    v1: Point = load v0
    return v1
}

function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: Point = aggregate (v0)
    v2: ref<Point, borrowed, mutable, static> = global.address test.main.ORIGIN
    store v2, v1
    return
}

/// @layout.struct name=Point size=4 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
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
    drop(&exclusive this): void {}
}

function consume(guard: Guard<int32>): void {
    drop(guard);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Guard<int32> {
    value: int32;
}

function test.main.consume(v0: Guard<int32>): void {
entry(v0: Guard<int32>):
    drop v0
    return
}

function test.main.drop<int32, 'a>(v0: ref<Guard<int32>, borrowed, 'a, exclusive, local>): void {
entry(v0: ref<Guard<int32>, borrowed, 'a, exclusive, local>):
    return
}

/// @layout.struct name=Guard<int32> size=4 align=4
/// @layout.field owner=Guard<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );
}
