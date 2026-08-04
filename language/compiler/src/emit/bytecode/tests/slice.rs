use crate::tests::TestProgram;

/// Emit slice subviews and direct length projection from the physical descriptor.
#[test]
fn test_emit_bytecode_slice() {
    let program = TestProgram::mir(
        r#"
export function subview(
    v0: slice<int32, borrowed, readonly>,
    v1: uint64,
    v2: uint64,
): uint64 {
entry(v0: slice<int32, borrowed, readonly>, v1: uint64, v2: uint64):
    v3: slice<int32, borrowed, readonly> = slice.view v0, v1, v2
    v4: uint64 = slice.length v3
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function subview {
    slice.view r4:r5, r0:r1, 4, r2, r3
    extract r0, r4:r5, 8:8
    return r0
}
"#,
    );
}
