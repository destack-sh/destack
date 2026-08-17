use crate::tests::TestSession;

#[test]
fn test_lower_field_read_to_field_get() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function sum(point: Point): int32 {
    return point.x + point.y;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

function test.main.sum(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 1
    v3: int32 = int.add v1, v2
    return v3
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nested_field_read_chain() {
    let session = TestSession::single(
        r#"
struct Size {
    width: int32;
    height: int32;
}

struct Frame {
    corner: int32;
    size: Size;
}

function area(frame: Frame): int32 {
    return frame.size.width * frame.size.height;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Size {
    width: int32;
    height: int32;
}

@copy
type Frame {
    corner: int32;
    size: Size;
}

function test.main.area(v0: Frame): int32 {
entry(v0: Frame):
    v1: Size = field.get v0, 1
    v2: int32 = field.get v1, 0
    v3: Size = field.get v0, 1
    v4: int32 = field.get v3, 1
    v5: int32 = int.mul v2, v4
    return v5
}

/// @layout.struct name=Size size=8 align=4
/// @layout.field owner=Size index=0 name=width offset=0 size=4 align=4
/// @layout.field owner=Size index=1 name=height offset=4 size=4 align=4
/// @layout.struct name=Frame size=12 align=4
/// @layout.field owner=Frame index=0 name=corner offset=0 size=4 align=4
/// @layout.field owner=Frame index=1 name=size offset=4 size=8 align=4
"#,
    );
}

#[test]
fn test_lower_a_borrow_of_a_member_place() {
    let session = TestSession::single(
        r#"
struct Meter {
    name: string;
}

function read(meter: &readonly Meter): &readonly string {
    return &readonly meter.name;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

@copy
type Meter {
    name: ref<destack.string.string.String, managed, mutable>;
}

function test.main.read<'a>(v0: ref<Meter, borrowed, 'a, readonly>): ref<destack.string.string.String, borrowed, 'a, readonly> {
entry(v0: ref<Meter, borrowed, 'a, readonly>):
    v1: ref<ref<destack.string.string.String, managed, readonly>, borrowed, readonly> = field.address v0, 0
    v2: ref<destack.string.string.String, managed, readonly> = load v1
    v3: ref<destack.string.string.String, borrowed, 'a, readonly> = cast.bit v2 -> ref<destack.string.string.String, borrowed, 'a, readonly>
    return v3
}

/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
/// @layout.struct name=Meter size=8 align=8
/// @layout.field owner=Meter index=0 name=name offset=0 size=8 align=8
"#,
    );
}
