use crate::tests::TestSession;

#[test]
fn test_lower_if_return_to_branch_with_join() {
    let session = TestSession::single(
        r#"
function max(a: int32, b: int32): int32 {
    if (a > b) {
        return a;
    }
    return b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.max(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: boolean = int.gt.s v0, v1
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_if_else_with_both_arms_returning() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean, a: int32, b: int32): int32 {
    if (flag) {
        return a;
    } else {
        return b;
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.pick(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0, b1, b3

b1:
    return v1

b2:
    unreachable

b3:
    return v2
}
"#,
    );
}

#[test]
fn test_lower_ternary_joins_arm_values() {
    let session = TestSession::single(
        r#"
function clamp(value: float32, limit: float32): float32 {
    return value > limit ? limit : value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.clamp(v0: float32, v1: float32): float32 {
    local l0: float32

entry(v0: float32, v1: float32):
    v2: boolean = float.gt v0, v1
    branch v2, b1, b2

b1:
    local.set l0, v1
    jump b3

b2:
    local.set l0, v0
    jump b3

b3:
    v3: float32 = local.get l0
    return v3
}
"#,
    );
}
