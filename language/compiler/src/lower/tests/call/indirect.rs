use crate::tests::TestSession;

/// Bind one free function into a function value and call it indirectly.
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

    session.assert_mir_function("main.tspp", "test.main.apply", r#"
function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
    local l0: function<(int32) => int32, repeatable, managed, mutable, local>
    local l1: int32

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    store l0, v0
    store l1, v1
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = load l0
    v3: int32 = load l1
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    return v5
}
"#);

    session.assert_mir_function("main.tspp", "test.main.run", r#"
function test.main.run(): int32 {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.double, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}
"#);
}
