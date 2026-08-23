use crate::tests::TestSession;

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
global test.main.start: int32 = zeroInit

function test.main.seed(): int32 {
entry:
    v0: int32 = 41
    return v0
}

function test.main.run(): int32 {
entry:
    v0: ref<int32, borrowed, readonly, global> = global.address test.main.start
    v1: int32 = load v0
    return v1
}

function test.main.@init(): void {
entry:
    v0: int32 = call test.main.seed(): () => int32
    v1: int32 = 1
    v2: int32 = add v0, v1
    v3: ref<int32, borrowed, mutable, global> = global.address test.main.start
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

global test.main.ORIGIN: Point = zeroInit

function test.main.pick(): Point {
entry:
    v0: ref<Point, borrowed, readonly, global> = global.address test.main.ORIGIN
    v1: Point = load v0
    return v1
}

function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: Point = aggregate (v0)
    v2: ref<Point, borrowed, mutable, global> = global.address test.main.ORIGIN
    store v2, v1
    return
}

/// @layout.struct name=Point size=4 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}
