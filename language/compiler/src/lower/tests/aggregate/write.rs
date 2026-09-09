use crate::tests::TestSession;

#[test]
fn test_store_a_field_write_through_its_address() {
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

    session.assert_mir_function(
        "main.ds",
        "test.main.shift",
        r#"
@copy
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.shift(v0: test.main.Point, v1: int32): test.main.Point {
    local l0: test.main.Point
    local l1: int32
    local l2: test.main.Point

entry(v0: test.main.Point, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.Point = local.get l0
    local.set l2, v2
    v3: test.main.Point = local.get l2
    v4: int32 = field.get v3, 0
    v5: int32 = local.get l1
    v6: int32 = add v4, v5
    v7: ref<test.main.Point, borrowed, 'frame, mutable, frame> = local.project l2
    v8: ref<int32, borrowed, 'frame, mutable, frame> = field.project v7, 0
    store v8, v6
    v9: test.main.Point = local.get l2
    return v9
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.widen",
        r#"
@copy
type test.main.Size {
    width: int32;
    height: int32;
}

@copy
type test.main.Frame {
    corner: int32;
    size: test.main.Size;
}

function test.main.widen(v0: test.main.Frame, v1: int32): test.main.Frame {
    local l0: test.main.Frame
    local l1: int32
    local l2: test.main.Frame

entry(v0: test.main.Frame, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.Frame = local.get l0
    local.set l2, v2
    v3: test.main.Frame = local.get l2
    v4: test.main.Size = field.get v3, 1
    v5: int32 = field.get v4, 0
    v6: int32 = local.get l1
    v7: int32 = add v5, v6
    v8: ref<test.main.Frame, borrowed, 'frame, mutable, frame> = local.project l2
    v9: ref<test.main.Size, borrowed, 'frame, mutable, frame> = field.project v8, 1
    v10: ref<int32, borrowed, 'frame, mutable, frame> = field.project v9, 0
    store v10, v7
    v11: test.main.Frame = local.get l2
    return v11
}

/// @layout.struct name=test.main.Size size=8 align=4
/// @layout.field owner=test.main.Size index=0 name=width offset=0 size=4 align=4
/// @layout.field owner=test.main.Size index=1 name=height offset=4 size=4 align=4
/// @layout.struct name=test.main.Frame size=12 align=4
/// @layout.field owner=test.main.Frame index=0 name=corner offset=0 size=4 align=4
/// @layout.field owner=test.main.Frame index=1 name=size offset=4 size=8 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.tick",
        r#"
@copy
type test.main.Counter {
    hits: int32;
}

function test.main.tick(v0: test.main.Counter): test.main.Counter {
    local l0: test.main.Counter
    local l1: test.main.Counter

entry(v0: test.main.Counter):
    local.set l0, v0
    v1: test.main.Counter = local.get l0
    local.set l1, v1
    v2: test.main.Counter = local.get l1
    v3: int32 = field.get v2, 0
    v4: int32 = 1
    v5: int32 = add v3, v4
    v6: ref<test.main.Counter, borrowed, 'frame, mutable, frame> = local.project l1
    v7: ref<int32, borrowed, 'frame, mutable, frame> = field.project v6, 0
    store v7, v5
    v8: test.main.Counter = local.get l1
    return v8
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=hits offset=0 size=4 align=4
"#,
    );
}
