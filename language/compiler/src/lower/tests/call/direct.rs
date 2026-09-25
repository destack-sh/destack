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

    session.assert_mir_function(
        "main.tspp",
        "test.main.double",
        r#"
function test.main.double(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
    v3: int32 = add v1, v2
    return v3
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.quad",
        r#"
function test.main.quad(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = call test.main.double(v1): (int32) => int32
    v3: int32 = call test.main.double(v2): (int32) => int32
    return v3
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.noop",
        r#"
function test.main.noop(): void {
entry:
    return
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
function test.main.run(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    call test.main.noop(): () => void
    v1: int32 = load l0
    return v1
}
"#,
    );
}
