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
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.shift(v0: test.main.Point, v1: int32): test.main.Point {
    local l0: test.main.Point
    local l1: int32
    local l2: test.main.Point

entry(v0: test.main.Point, v1: int32):
    store l0, v0
    store l1, v1
    v2: test.main.Point = load l0
    store l2, v2
    v3: int32 = load (l2).0
    v4: int32 = load l1
    v5: int32 = add v3, v4
    store (l2).0, v5
    v6: test.main.Point = load l2
    return v6
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
type test.main.Frame {
    corner: int32;
    size: test.main.Size;
}

function test.main.widen(v0: test.main.Frame, v1: int32): test.main.Frame {
    local l0: test.main.Frame
    local l1: int32
    local l2: test.main.Frame

entry(v0: test.main.Frame, v1: int32):
    store l0, v0
    store l1, v1
    v2: test.main.Frame = load l0
    store l2, v2
    v3: int32 = load ((l2).1).0
    v4: int32 = load l1
    v5: int32 = add v3, v4
    store ((l2).1).0, v5
    v6: test.main.Frame = load l2
    return v6
}

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
type test.main.Counter {
    hits: int32;
}

function test.main.tick(v0: test.main.Counter): test.main.Counter {
    local l0: test.main.Counter
    local l1: test.main.Counter

entry(v0: test.main.Counter):
    store l0, v0
    v1: test.main.Counter = load l0
    store l1, v1
    v2: int32 = load (l1).0
    v3: int32 = 1
    v4: int32 = add v2, v3
    store (l1).0, v4
    v5: test.main.Counter = load l1
    return v5
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=hits offset=0 size=4 align=4
"#,
    );
}
