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
function main.swap(v0: (int32, float64)): (float64, int32) {
entry(v0: (int32, float64)):
    v1: float64 = field.get v0, 1
    v2: int32 = field.get v0, 0
    v3: (float64, int32) = aggregate (v1, v2)
    return v3
}
/// @layout.tuple name=type@2 size=16 align=8 elements=(@8+4, @0+8)
/// @layout.tuple name=type@5 size=16 align=8 elements=(@0+8, @8+4)
/// @layout.tuple name=type@11 size=16 align=8 elements=(@0+8, @8+4)
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

function main.corners(v0: Point, v1: Point): (Point, Point) {
entry(v0: Point, v1: Point):
    v2: (Point, Point) = aggregate (v0, v1)
    return v2
}
/// @layout.struct name=Point size=8 align=4 fields=(x@0+4, y@4+4)
/// @layout.tuple name=type@6 size=16 align=4 elements=(@0+8, @8+8)
/// @layout.tuple name=type@10 size=16 align=4 elements=(@0+8, @8+8)
"#,
    );
}

