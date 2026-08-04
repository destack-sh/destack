use crate::tests::TestProgram;

/// Emit tagged variant construction, tag access, and payload projection.
#[test]
fn test_emit_bytecode_variant() {
    let program = TestProgram::mir(
        r#"
export function payload(v0: int32): int32 {
entry(v0: int32):
    v1: variant<uint8> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v0
    v2: uint8 = variant.tag v1
    v3: int32 = variant.payload v1, 0
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function payload {
    variant.new r1, l4, 0, r0
    variant.tag r0, r1, l4
    extract r0, r1, 4:4
    return r0
}
"#,
    );
}
