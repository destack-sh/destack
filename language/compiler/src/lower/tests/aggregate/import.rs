use crate::tests::TestSession;

#[test]
fn test_lower_imported_struct_construction_and_field_reads() {
    let session = TestSession::builder()
        .module(
            "point.tspp",
            r#"
export struct Point {
    x: int32;
    y: int32;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Point } from "./point";

function stretch(by: int32): int32 {
    let point: Point = Point { x: by, y: by };
    return point.x + point.y;
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.tspp",
        "test.main.stretch",
        r#"
type test.point.Point;

function test.main.stretch(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
    v3: test.point.Point = aggregate (v1, v2)
    store l1, v3
    v4: int32 = load (l1).0
    v5: int32 = load (l1).1
    v6: int32 = add v4, v5
    return v6
}
"#,
    );
}
