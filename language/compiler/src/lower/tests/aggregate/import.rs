use crate::tests::TestSession;

#[test]
fn test_lower_imported_struct_construction_and_field_reads() {
    let session = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;
}
"#,
        )
        .module(
            "main.ds",
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
        "main.ds",
        "test.main.stretch",
        r#"
@copy
type test.point.Point;

function test.main.stretch(v0: int32): int32 {
    local l0: int32
    local l1: test.point.Point

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: test.point.Point = aggregate (v1, v2)
    local.set l1, v3
    v4: test.point.Point = local.get l1
    v5: int32 = field.get v4, 0
    v6: test.point.Point = local.get l1
    v7: int32 = field.get v6, 1
    v8: int32 = add v5, v7
    return v8
}
"#,
    );
}
