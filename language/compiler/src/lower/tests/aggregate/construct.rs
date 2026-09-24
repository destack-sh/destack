use crate::tests::TestSession;

#[test]
fn test_lower_struct_construction_to_aggregate() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function diagonal(a: int32, b: int32): Point {
    return Point { x: a, y: b };
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.diagonal",
        r#"
type test.main.Point {
    x: int32;
    y: int32;
}

function test.main.diagonal(v0: int32, v1: int32): test.main.Point {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: test.main.Point = aggregate (v2, v3)
    return v4
}

/// @layout.struct name=test.main.Point size=8 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=test.main.Point index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_struct_construction_across_field_widths() {
    let session = TestSession::single(
        r#"
struct Sample {
    flag: boolean;
    weight: float64;
    count: int32;
}

function sample(flag: boolean, weight: float64, count: int32): Sample {
    return Sample { flag: flag, weight: weight, count: count };
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.sample",
        r#"
type test.main.Sample {
    flag: boolean;
    weight: float64;
    count: int32;
}

function test.main.sample(v0: boolean, v1: float64, v2: int32): test.main.Sample {
    local l0: boolean
    local l1: float64
    local l2: int32

entry(v0: boolean, v1: float64, v2: int32):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: boolean = load l0
    v4: float64 = load l1
    v5: int32 = load l2
    v6: test.main.Sample = aggregate (v3, v4, v5)
    return v6
}

/// @layout.struct name=test.main.Sample size=16 align=8
/// @layout.field owner=test.main.Sample index=0 name=flag offset=12 size=1 align=1
/// @layout.field owner=test.main.Sample index=1 name=weight offset=0 size=8 align=8
/// @layout.field owner=test.main.Sample index=2 name=count offset=8 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_unused_concrete_nominal_declaration() {
    let session = TestSession::single(
        r#"
struct Metadata {
    value: int32;
}

function identity(value: int32): int32 {
    return value;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.identity",
        r#"
function test.main.identity(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    return v1
}
"#,
    );
}
