use crate::tests::TestProgram;

/// Emit fixed and repeated initialized and uninitialized allocations.
#[test]
fn test_emit_bytecode_allocation() {
    let program = TestProgram::mir(
        r#"
export function allocate(v0: int64): slice<int32, unique, mutable> {
entry(v0: int64):
    v1: ref<int32, unique, mutable> = new.zeroed int32
    v2: uninit<ref<int32, unique, mutable>> = new.uninit int32
    v3: ref<int32, unique, mutable> = new.complete v2
    v4: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    v5: uninit<slice<int32, unique, mutable>> = new.slice.uninit int32, v0
    v6: slice<int32, unique, mutable> = new.complete v5
    return v6
}
"#,
    );

    program.assert_bytecode(
        r#"
function allocate {
    new.zeroed r1, a0: ref<unique, local>
    new.uninit r1, a1: ref<unique, local>
    move r2, r1
    new.slice.zeroed r1:r2, a2, r0: ref<unique, local>
    new.slice.uninit r1:r2, a3, r0: ref<unique, local>
    move r3:r4, r1:r2
    return r3:r4
}
"#,
    );
}
