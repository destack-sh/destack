use crate::tests::TestSession;

#[test]
fn test_lower_direct_call_between_local_functions() {
    let session = TestSession::single(
        r#"
function double(x: int32): int32 {
    return x + x;
}

function quad(x: int32): int32 {
    return double(double(x));
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.double(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function test.main.quad(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call test.main.double(v0): (int32) => int32
    v2: int32 = call test.main.double(v1): (int32) => int32
    return v2
}
"#,
    );
}

#[test]
fn test_lower_void_call_as_statement() {
    let session = TestSession::single(
        r#"
function noop(): void {}

function run(x: int32): int32 {
    noop();
    return x;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.noop(): void {
entry:
    return
}

function test.main.run(v0: int32): int32 {
entry(v0: int32):
    call test.main.noop(): () => void
    return v0
}
"#,
    );
}
