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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Point {
    x: int32;
    y: int32;
}

function main.diagonal(v0: int32, v1: int32): Point {
entry(v0: int32, v1: int32):
    v2: Point = aggregate (v0, v1)
    return v2
}
/// @layout.struct name=Point size=8 align=4
/// @layout.field owner=Point index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=Point index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Sample {
    flag: boolean;
    weight: float64;
    count: int32;
}

function main.sample(v0: boolean, v1: float64, v2: int32): Sample {
entry(v0: boolean, v1: float64, v2: int32):
    v3: Sample = aggregate (v0, v1, v2)
    return v3
}
/// @layout.struct name=Sample size=16 align=8
/// @layout.field owner=Sample index=0 name=flag offset=12 size=1 align=1
/// @layout.field owner=Sample index=1 name=weight offset=0 size=8 align=8
/// @layout.field owner=Sample index=2 name=count offset=8 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Metadata {
    value: int32;
}

function main.identity(v0: int32): int32 {
entry(v0: int32):
    return v0
}
/// @layout.struct name=Metadata size=4 align=4
/// @layout.field owner=Metadata index=0 name=value offset=0 size=4 align=4
"#,
    );
}
