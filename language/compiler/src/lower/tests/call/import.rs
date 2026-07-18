use crate::tests::TestSession;

#[test]
fn test_lower_imported_function_call_to_an_extern_symbol() {
    let session = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math";

function total(base: int32): int32 {
    return add(base, base);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.total(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call math.add(v0, v0)
    return v1
}

external function math.add(int32, int32): int32
"#,
    );
}

#[test]
fn test_lower_imported_call_returning_a_foreign_struct() {
    let session = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;
}

export function diagonal(a: int32, b: int32): Point {
    return Point { x: a, y: b };
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point, diagonal } from "./point";

function stretch(by: int32): int32 {
    let point: Point = diagonal(by, by);
    return point.x + point.y;
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type point.Point {
    x: int32;
    y: int32;
}

function main.stretch(v0: int32): int32 {
    local l0: point.Point

entry(v0: int32):
    v1: point.Point = call point.diagonal(v0, v0)
    local.set l0, v1
    v2: point.Point = local.get l0
    v3: int32 = field.get v2, 0
    v4: point.Point = local.get l0
    v5: int32 = field.get v4, 1
    v6: int32 = int.add v3, v5
    return v6
}

external function point.diagonal(int32, int32): point.Point
/// @layout.struct name=point.Point size=8 align=4 fields=(x@0+4, y@4+4)
"#,
    );
}
