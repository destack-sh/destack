use crate::tests::TestSession;

#[test]
fn test_lower_float_arithmetic_with_literal() {
    let session = TestSession::single(
        r#"
function scale(x: float64, factor: float64): float64 {
    return x * factor + 1.5;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.scale(v0: float64, v1: float64): float64 {
entry(v0: float64, v1: float64):
    v2: float64 = float.mul v0, v1
    v3: float64 = 1.5
    v4: float64 = float.add v2, v3
    return v4
}
"#,
    );
}

#[test]
fn test_lower_float_ordering_to_float_compare() {
    let session = TestSession::single(
        r#"
function hotter(x: float32, limit: float32): boolean {
    return x > limit;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.hotter(v0: float32, v1: float32): boolean {
entry(v0: float32, v1: float32):
    v2: boolean = float.gt v0, v1
    return v2
}
"#,
    );
}
