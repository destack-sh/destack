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
    v1: float64 = add v0, v0
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
    v3: int32 = add v1, v2
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

#[test]
fn test_lower_destructuring_let_binds_projected_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function span(point: Point): int32 {
    const { x, y } = point;
    return x + y;
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

function test.main.span(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 1
    v3: int32 = add v1, v2
    return v3
}

/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_required_binding_traps_on_absence() {
    let session = TestSession::single(
        r#"
function parse(length: int32): int32 | undefined {
    return length > 0 ? length : undefined;
}

function read(length: int32): int32 {
    const value! = parse(length);
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.parse(v0: int32): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }

entry(v0: int32):
    v1: int32 = 0
    v2: boolean = gt v0, v1
    branch v2 => b1 | b2

b1:
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    local.set l0, v3
    jump b3

b2:
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    local.set l0, v4
    jump b3

b3:
    v5: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = local.get l0
    return v5
}

function test.main.read(v0: int32): int32 {
    local l0: int32, readonly

entry(v0: int32):
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = call test.main.parse(v0): (int32) => variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    variant.switch v1, 1 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 0
    local.set l0, v2
    jump b3

b2:
    unreachable

b3:
    v3: int32 = local.get l0
    return v3
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );
}
