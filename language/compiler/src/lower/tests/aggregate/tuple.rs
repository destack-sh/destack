use crate::tests::TestSession;

#[test]
fn test_lower_tuple_construction_and_element_reads() {
    let session = TestSession::single(
        r#"
function swap(pair: (int32, float64)): (float64, int32) {
    return (pair[1], pair[0]);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.swap(v0: (int32, float64)): (float64, int32) {
entry(v0: (int32, float64)):
    v1: float64 = field.get v0, 1
    v2: int32 = field.get v0, 0
    v3: (float64, int32) = aggregate (v1, v2)
    return v3
}
/// @layout.tuple name=type@2 size=16 align=8
/// @layout.element owner=type@2 index=0 offset=8 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=0 size=8 align=8
/// @layout.tuple name=type@3 size=16 align=8
/// @layout.element owner=type@3 index=0 offset=0 size=8 align=8
/// @layout.element owner=type@3 index=1 offset=8 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_tuple_of_structs() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function corners(a: Point, b: Point): (Point, Point) {
    return (a, b);
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

function test.main.corners(v0: Point, v1: Point): (Point, Point) {
entry(v0: Point, v1: Point):
    v2: (Point, Point) = aggregate (v0, v1)
    return v2
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
/// @layout.tuple name=type@5 size=16 align=4
/// @layout.element owner=type@5 index=0 offset=0 size=8 align=4
/// @layout.element owner=type@5 index=1 offset=8 size=8 align=4
"#,
    );
}
