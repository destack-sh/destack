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
/// @layout.struct name=Point size=8 align=4 fields=(x@0+4, y@4+4)
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
/// @layout.struct name=Sample size=16 align=8 fields=(flag@12+1, weight@0+8, count@8+4)
"#,
    );
}
