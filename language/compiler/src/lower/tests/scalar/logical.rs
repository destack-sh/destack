use crate::tests::TestSession;

#[test]
fn test_short_circuit_a_logical_and() {
    let session = TestSession::single(
        r#"
function both(a: boolean, b: boolean): boolean {
    return a && b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.both(v0: boolean, v1: boolean): boolean {
    local l0: boolean

entry(v0: boolean, v1: boolean):
    local.set l0, v0
    branch v0 => b1 | b2

b1:
    local.set l0, v1
    jump b2

b2:
    v2: boolean = local.get l0
    return v2
}
"#,
    );
}

#[test]
fn test_short_circuit_a_logical_or() {
    let session = TestSession::single(
        r#"
function either(a: boolean, b: boolean): boolean {
    return a || b;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.either(v0: boolean, v1: boolean): boolean {
    local l0: boolean

entry(v0: boolean, v1: boolean):
    local.set l0, v0
    branch v0 => b2 | b1

b1:
    local.set l0, v1
    jump b2

b2:
    v2: boolean = local.get l0
    return v2
}
"#,
    );
}
