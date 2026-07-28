use crate::tests::TestSession;

#[test]
fn test_lower_const_binding_without_local() {
    let session = TestSession::single(
        r#"
function twice(x: float64): float64 {
    const y = x + x;
    return y;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.twice(v0: float64): float64 {
entry(v0: float64):
    v1: float64 = float.add v0, v0
    return v1
}
"#,
    );
}
#[test]
fn test_lower_mutable_binding_through_local() {
    let session = TestSession::single(
        r#"
function bump(x: int32): int32 {
    let y = x;
    y = y + 1;
    return y;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.bump(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = 1
    v3: int32 = int.add v1, v2
    local.set l0, v3
    v4: int32 = local.get l0
    return v4
}
"#,
    );
}

#[test]
fn test_lower_owned_value_forwarding_between_bindings() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function relay(): int32 {
    let point: ^Point = Point { x: 3, y: 4 };
    const taken = point;
    return taken.x;
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

function test.main.relay(): int32 {
    local l0: Point

entry:
    v0: int32 = 3
    v1: int32 = 4
    v2: Point = aggregate (v0, v1)
    local.set l0, v2
    v3: Point = local.get l0
    v4: int32 = field.get v3, 0
    return v4
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}
