use crate::tests::TestSession;

#[test]
fn test_lower_float_arithmetic_with_literal() {
    let session = TestSession::single(
        r#"
function scale(x: float64, factor: float64): float64 {
    return (x * factor + 1.5) % factor;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.scale",
        r#"
function test.main.scale(v0: float64, v1: float64): float64 {
    local l0: float64
    local l1: float64

entry(v0: float64, v1: float64):
    store l0, v0
    store l1, v1
    v2: float64 = load l0
    v3: float64 = load l1
    v4: float64 = mul v2, v3
    v5: float64 = 1.5
    v6: float64 = add v4, v5
    v7: float64 = load l1
    v8: float64 = rem v6, v7
    return v8
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

    session.assert_mir_function(
        "main.ds",
        "test.main.hotter",
        r#"
function test.main.hotter(v0: float32, v1: float32): boolean {
    local l0: float32
    local l1: float32

entry(v0: float32, v1: float32):
    store l0, v0
    store l1, v1
    v2: float32 = load l0
    v3: float32 = load l1
    v4: boolean = gt v2, v3
    return v4
}
"#,
    );
}
