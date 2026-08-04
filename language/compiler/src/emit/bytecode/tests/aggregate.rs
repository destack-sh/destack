use crate::tests::TestProgram;

/// Emit aggregate construction, local transfer, update, and projection.
#[test]
fn test_emit_bytecode_aggregate() {
    let program = TestProgram::mir(
        r#"
type Pair {
    left: int32;
    right: int32;
}

export function transform(v0: int32, v1: int32): int32 {
    local l0: Pair

entry(v0: int32, v1: int32):
    v2: Pair = aggregate (v0, v1)
    v3: Pair = field.set v2, 1, v0
    local.set l0, v3
    v4: Pair = local.get l0
    v5: int32 = field.get v4, 1
    return v5
}
"#,
    );
    program.assert_bytecode(
        r#"
function transform {
    aggregate r2, [r0 @ 0:4, r1 @ 4:4]
    insert r1, r2, 4:4, r0
    move r3, r1
    move r0, r3
    extract r1, r0, 4:4
    return r1
}
"#,
    );
}
