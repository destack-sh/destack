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

    session.assert_mir_function(
        "main.tspp",
        "test.main.swap",
        r#"
function test.main.swap(v0: (int32, float64)): (float64, int32) {
    local l0: (int32, float64)

entry(v0: (int32, float64)):
    store l0, v0
    v1: (int32, float64) = load l0
    v2: float64 = field.get v1, 1
    v3: (int32, float64) = load l0
    v4: int32 = field.get v3, 0
    v5: (float64, int32) = aggregate (v2, v4)
    return v5
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.corners",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.corners(v0: test.main.Point, v1: test.main.Point): (test.main.Point, test.main.Point) {
    local l0: test.main.Point
    local l1: test.main.Point

entry(v0: test.main.Point, v1: test.main.Point):
    store l0, v0
    store l1, v1
    v2: test.main.Point = load l0
    v3: test.main.Point = load l1
    v4: (test.main.Point, test.main.Point) = aggregate (v2, v3)
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
/// @layout.tuple name=type@3 size=16 align=4
/// @layout.element owner=type@3 index=0 offset=0 size=8 align=4
/// @layout.element owner=type@3 index=1 offset=8 size=8 align=4
"#,
    );
}
