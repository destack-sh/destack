use crate::tests::TestSession;

#[test]
fn test_lower_field_write_rebuilds_aggregate() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function shift(start: Point, by: int32): Point {
    let point = start;
    point.x = point.x + by;
    return point;
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

function test.main.shift(v0: Point, v1: int32): Point {
    local l0: Point

entry(v0: Point, v1: int32):
    local.set l0, v0
    v2: Point = local.get l0
    v3: int32 = field.get v2, 0
    v4: int32 = int.add v3, v1
    v5: Point = local.get l0
    v6: Point = field.set v5, 0, v4
    local.set l0, v6
    v7: Point = local.get l0
    return v7
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_nested_field_write_chain() {
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

function widen(frame: Frame, by: int32): Frame {
    let updated = frame;
    updated.size.width += by;
    return updated;
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

function test.main.widen(v0: Frame, v1: int32): Frame {
    local l0: Frame

entry(v0: Frame, v1: int32):
    local.set l0, v0
    v2: Frame = local.get l0
    v3: Size = field.get v2, 1
    v4: int32 = field.get v3, 0
    v5: int32 = int.add v4, v1
    v6: Frame = local.get l0
    v7: Size = field.get v6, 1
    v8: Size = field.set v7, 0, v5
    v9: Frame = field.set v6, 1, v8
    local.set l0, v9
    v10: Frame = local.get l0
    return v10
}
/// @layout.struct name=Size size=8 align=4
/// @layout.field owner=Size index=0 name=width offset=0 size=4 align=4
/// @layout.field owner=Size index=1 name=height offset=4 size=4 align=4
/// @layout.struct name=Frame size=12 align=4
/// @layout.field owner=Frame index=0 name=corner offset=0 size=4 align=4
/// @layout.field owner=Frame index=1 name=size offset=4 size=8 align=4
"#,
    );
}

#[test]
fn test_lower_field_update_through_place() {
    let session = TestSession::single(
        r#"
struct Counter {
    hits: int32;
}

function tick(counter: Counter): Counter {
    let updated = counter;
    updated.hits++;
    return updated;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Counter {
    hits: int32;
}

function test.main.tick(v0: Counter): Counter {
    local l0: Counter

entry(v0: Counter):
    local.set l0, v0
    v1: Counter = local.get l0
    v2: int32 = field.get v1, 0
    v3: int32 = 1
    v4: int32 = int.add v2, v3
    v5: Counter = local.get l0
    v6: Counter = field.set v5, 0, v4
    local.set l0, v6
    v7: Counter = local.get l0
    return v7
}
/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=hits offset=0 size=4 align=4
"#,
    );
}
