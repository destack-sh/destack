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
function twice(v0: float64): float64 {
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
function bump(v0: int32): int32 {
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
