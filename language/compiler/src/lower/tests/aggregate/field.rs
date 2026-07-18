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

function main.sum(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 1
    v3: int32 = int.add v1, v2
    return v3
}
/// @layout.struct name=Point size=8 align=4 fields=(x@0+4, y@4+4)
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

function main.area(v0: Frame): int32 {
entry(v0: Frame):
    v1: Size = field.get v0, 1
    v2: int32 = field.get v1, 0
    v3: Size = field.get v0, 1
    v4: int32 = field.get v3, 1
    v5: int32 = int.mul v2, v4
    return v5
}
/// @layout.struct name=Size size=8 align=4 fields=(width@0+4, height@4+4)
/// @layout.struct name=Frame size=12 align=4 fields=(corner@0+4, size@4+8)
"#,
    );
}
