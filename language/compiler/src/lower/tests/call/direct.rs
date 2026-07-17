use crate::tests::TestSession;

#[test]
fn test_lower_direct_call_between_module_functions() {
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
function double(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function quad(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call double(v0)
    v2: int32 = call double(v1)
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
function noop(): void {
entry:
    return
}

function run(v0: int32): int32 {
entry(v0: int32):
    call noop()
    return v0
}
"#,
    );
}
