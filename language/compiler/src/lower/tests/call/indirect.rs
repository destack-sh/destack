use crate::tests::TestSession;

#[test]
fn test_bind_and_call_a_function_value() {
    let session = TestSession::single(
        r#"
function double(x: int32): int32 {
    return x + x;
}

function apply(f: (x: int32) => int32, v: int32): int32 {
    return f(v);
}

function run(): int32 {
    return apply(double, 7);
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

function test.main.apply(v0: (int32) => int32, v1: int32): int32 {
entry(v0: (int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}

function test.main.run(): int32 {
entry:
    v0: ref<{  }, managed, mutable, nullable> = null
    v1: (int32) => int32 = function.bind test.main.double, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2)
    return v3
}
/// @layout.struct name=type@3 size=0 align=1
"#,
    );
}
