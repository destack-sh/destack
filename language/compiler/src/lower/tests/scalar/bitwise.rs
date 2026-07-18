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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.nudge(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 2
    v2: int32 = int.shl v0, v1
    return v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.mask(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    v2: uint32 = int.and v0, v1
    v3: uint32 = int.xor v0, v1
    v4: uint32 = int.or v2, v3
    return v4
}
"#,
    );
}
