use crate::tests::TestSession;

/// Lower an object literal as an instance of its declared class.
#[test]
fn test_lower_an_object_literal_to_its_declared_class() {
    let session = TestSession::single(
        r#"
type Point = { x: int32 };

function read(point: Point): int32 {
    return point.x;
}

function build(): int32 {
    const point: Point = { x: 7 };
    return read(point);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Point {
    x: int32;
}

function test.main.read(v0: ref<Point, managed, mutable>): int32 {
entry(v0: ref<Point, managed, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.build(): int32 {
entry:
    v0: int32 = 7
    v1: Point = aggregate (v0)
    v2: ref<Point, managed, mutable> = new.complete v1
    v3: int32 = call test.main.read(v2): (ref<Point, managed, mutable>) => int32
    return v3
}

/// @layout.struct name=Point size=4 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// Lower a recursive object alias to its named managed reference.
#[test]
fn test_lower_a_recursive_object_alias_to_a_named_reference() {
    let session = TestSession::single(
        r#"
type Selector = {
    depth?: int32;
    nested?: Selector;
};

function pick(selector: Selector): int32 {
    return 0;
}

function build(): int32 {
    return pick({ depth: 3 });
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Selector {
    depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
    nested: ref<Selector, managed, mutable, undefined>;
}

function test.main.pick(v0: ref<Selector, managed, mutable>): int32 {
entry(v0: ref<Selector, managed, mutable>):
    v1: int32 = 0
    return v1
}

function test.main.build(): int32 {
entry:
    v0: int32 = 3
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: ref<Selector, managed, mutable, undefined> = undefined
    v3: Selector = aggregate (v1, v2)
    v4: ref<Selector, managed, mutable> = new.complete v3
    v5: int32 = call test.main.pick(v4): (ref<Selector, managed, mutable>) => int32
    return v5
}

/// @layout.struct name=Selector size=16 align=8
/// @layout.field owner=Selector index=0 name=depth offset=8 size=8 align=4
/// @layout.field owner=Selector index=1 name=nested offset=0 size=8 align=8
/// @layout.variant name=type@5 size=8 align=4
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=4
"#,
    );
}

/// Construct an owned object literal in place without an allocation.
#[test]
fn test_construct_an_owned_object_literal_in_place() {
    let session = TestSession::single(
        r#"
function build(): int32 {
    const point: ^{ x: int32 } = { x: 7 };
    return point.x;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.build(): int32 {
entry:
    v0: int32 = 7
    v1: { x: int32 } = aggregate (v0)
    v2: int32 = field.get v1, 0
    return v2
}

/// @layout.struct name=type@5 size=4 align=4
/// @layout.field owner=type@5 index=0 name=x offset=0 size=4 align=4
"#,
    );
}
