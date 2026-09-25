use crate::tests::TestSession;

#[test]
fn test_lower_shift_with_literal_amount() {
    let session = TestSession::single(
        r#"
function nudge(x: int32): int32 {
    return x << 2;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.nudge",
        r#"
function test.main.nudge(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = 2
    v3: int32 = shl v1, v2
    return v3
}
"#,
    );
}

#[test]
fn test_lower_bitwise_operations_over_unsigned_operands() {
    let session = TestSession::single(
        r#"
function mask(x: uint32, m: uint32): uint32 {
    return (x & m) | (x ^ m);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.mask",
        r#"
function test.main.mask(v0: uint32, v1: uint32): uint32 {
    local l0: uint32
    local l1: uint32

entry(v0: uint32, v1: uint32):
    store l0, v0
    store l1, v1
    v2: uint32 = load l0
    v3: uint32 = load l1
    v4: uint32 = and v2, v3
    v5: uint32 = load l0
    v6: uint32 = load l1
    v7: uint32 = xor v5, v6
    v8: uint32 = or v4, v7
    return v8
}
"#,
    );
}
