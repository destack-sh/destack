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
        "main.ds",
        "test.main.double",
        r#"
function test.main.double(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: int32 = add v1, v2
    return v3
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.apply", r#"
function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
    local l0: function<(int32) => int32, repeatable, managed, mutable, local>
    local l1: int32

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = local.get l0
    v3: int32 = local.get l1
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, mutable, local> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable, local>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    return v5
}
"#);

    session.assert_mir_function("main.ds", "test.main.run", r#"
function test.main.run(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.double, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}

/// @layout.variant name=type@30 size=8 align=8
/// @layout.discriminant owner=type@30 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@30 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@30 index=1 discriminant=1 payload_offset=0
"#);
}
