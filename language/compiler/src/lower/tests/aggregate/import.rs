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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.point.Point {
    x: int32;
    y: int32;
}

function test.main.stretch(v0: int32): int32 {
    local l0: test.point.Point

entry(v0: int32):
    v1: test.point.Point = aggregate (v0, v0)
    local.set l0, v1
    v2: test.point.Point = local.get l0
    v3: int32 = field.get v2, 0
    v4: test.point.Point = local.get l0
    v5: int32 = field.get v4, 1
    v6: int32 = add v3, v5
    return v6
}

/// @layout.struct name=test.point.Point size=8 align=4
/// @layout.field owner=test.point.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.point.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}
