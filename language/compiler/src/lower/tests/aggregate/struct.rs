use crate::tests::TestSession;

#[test]
fn test_lower_struct_literal_to_aggregate() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function origin(): Point {
    return Point { x: 0, y: 0 };
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

function main.origin(): Point {
entry:
    v0: int32 = 0
    v1: int32 = 0
    v2: Point = aggregate (v0, v1)
    return v2
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_struct_field_read() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function abscissa(point: Point): int32 {
    return point.x;
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

function main.abscissa(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    return v1
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nested_struct_literal_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

struct Segment {
    start: Point;
    end: Point;
}

function diagonal(size: int32): Segment {
    return Segment { start: Point { x: 0, y: 0 }, end: Point { x: size, y: size } };
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

@copy
type Segment {
    start: Point;
    end: Point;
}

function main.diagonal(v0: int32): Segment {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 0
    v3: Point = aggregate (v1, v2)
    v4: Point = aggregate (v0, v0)
    v5: Segment = aggregate (v3, v4)
    return v5
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
/// @layout.struct name=Segment size=16 align=4
/// @layout.field owner=Segment index=0 name=start offset=0 size=8 align=4
/// @layout.field owner=Segment index=1 name=end offset=8 size=8 align=4
"#,
    );
}
