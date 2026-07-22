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
/// @layout.struct name=point.Point size=8 align=4
/// @layout.field owner=point.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=point.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_imported_struct_method_call_through_an_extern() {
    let session = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
    y: int32;

    length(): int32 {
        return this.x + this.y;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./point";

function measure(by: int32): int32 {
    let point = Point { x: by, y: by };
    return point.length();
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

function main.measure(v0: int32): int32 {
    local l0: point.Point

entry(v0: int32):
    v1: point.Point = aggregate (v0, v0)
    local.set l0, v1
    v2: ref<point.Point, borrowed, exclusive> = local.address l0
    v3: int32 = call point.Point.length(v2)
    return v3
}

external function point.Point.length<L0: lifetime>(ref<point.Point, borrowed, lifetime(L0), exclusive>): int32
/// @layout.struct name=point.Point size=8 align=4
/// @layout.field owner=point.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=point.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_imported_class_construction_and_method_call() {
    let session = TestSession::builder()
        .module(
            "box.ds",
            r#"
export class Box {
    weight: int32 = 0;

    constructor(weight: int32) {
        this.weight = weight;
    }

    weigh(): int32 {
        return this.weight;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Box } from "./box";

function open(): int32 {
    const parcel = new Box(7);
    return parcel.weigh();
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
type box.Box {
    weight: int32;
}

function main.open(): int32 {
entry:
    v0: ref<box.Box, managed, mutable> = new.zeroed box.Box
    v1: ref<box.Box, borrowed, exclusive> = cast.bit v0 -> ref<box.Box, borrowed, exclusive>
    v2: int32 = 7
    call box.Box.constructor(v1, v2)
    v3: int32 = call box.Box.weigh(v0)
    return v3
}

external function box.Box.constructor(ref<box.Box, borrowed, exclusive>, int32): void

external function box.Box.weigh(ref<box.Box, managed, mutable>): int32
/// @layout.struct name=box.Box size=4 align=4
/// @layout.field owner=box.Box index=0 name=weight offset=0 size=4 align=4
"#,
    );
}
