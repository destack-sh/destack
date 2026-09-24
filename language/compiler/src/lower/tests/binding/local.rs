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

    session.assert_mir_function(
        "main.ds",
        "test.main.twice",
        r#"
function test.main.twice(v0: float64): float64 {
    local l0: float64
    local l1: float64

entry(v0: float64):
    store l0, v0
    v1: float64 = load l0
    v2: float64 = load l0
    v3: float64 = add v1, v2
    store l1, v3
    v4: float64 = load l1
    return v4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.bump",
        r#"
function test.main.bump(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    store l1, v1
    v2: int32 = load l1
    v3: int32 = 1
    v4: int32 = add v2, v3
    store l1, v4
    v5: int32 = load l1
    return v5
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

    session.assert_mir_function(
        "main.ds",
        "test.main.relay",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.relay(): int32 {
    local l0: test.main.Point
    local l1: test.main.Point

entry:
    v0: int32 = 3
    v1: int32 = 4
    v2: test.main.Point = aggregate (v0, v1)
    store l0, v2
    v3: test.main.Point = load l0
    store l1, v3
    v4: int32 = load (l1).0
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.span",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.span(v0: test.main.Point): int32 {
    local l0: test.main.Point
    local l1: int32
    local l2: int32

entry(v0: test.main.Point):
    store l0, v0
    v1: int32 = load (l0).0
    store l1, v1
    v2: int32 = load (l0).1
    store l2, v2
    v3: int32 = load l1
    v4: int32 = load l2
    v5: int32 = add v3, v4
    return v5
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.parse",
        r#"
function test.main.parse(v0: int32): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: int32
    local l1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = 0
    v3: boolean = gt v1, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = load l0
    v5: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v4
    store l1, v5
    jump b3

b2:
    v6: void = zeroed
    v7: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    store l1, v7
    jump b3

b3:
    v8: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l1
    return v8
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.read", r#"
function test.main.read(v0: int32): int32 {
    local l0: int32
    local l1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l2: int32, readonly
    local l3: int32
    local l4: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = call test.main.parse(v1): (int32) => variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    store l1, v2
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l1
    variant.switch v3, 1 => b2, else b1

b1:
    v4: int32 = variant.payload v3, 0
    store l2, v4
    jump b3

b2:
    unreachable

b3:
    v5: int32 = load l2
    store l3, v5
    v6: int32 = load l3
    store l4, v6
    v7: int32 = load l4
    return v7
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}
