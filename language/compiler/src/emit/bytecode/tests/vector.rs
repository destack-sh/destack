use crate::tests::TestProgram;

/// Emit every fixed-width vector operation family.
#[test]
fn test_emit_bytecode_vector_operations() {
    let program = TestProgram::mir(
        r#"
export function vectors(
    v0: vector<int32, 4>,
    v1: vector<int32, 4>,
    v2: vector<boolean, 4>,
    v3: int32,
): int32 {
entry(v0: vector<int32, 4>, v1: vector<int32, 4>, v2: vector<boolean, 4>, v3: int32):
    v4: vector<int32, 4> = vector.splat v3
    v5: int32 = vector.extract v4, v3
    v6: vector<int32, 4> = vector.insert v4, v3, v5
    v7: vector<int32, 4> = vector.shuffle v6, v1, [0, 5, 2, 7]
    v8: vector<int32, 4> = vector.select v2, v7, v0
    v9: vector<int32, 4> = int.add v0, v1
    v10: int32 = vector.reduce add, v9
    v11: vector<boolean, 4> = vector.compare int.lt.s, v8, v1
    v12: vector<int32, 4> = vector.convert exact, v8
    return v10
}
"#,
    );

    program.assert_bytecode(
        r#"
function vectors {
    vector.splat r6:r7, r5: vector<int32, 4>
    vector.extract r8, r6:r7, r5: vector<int32, 4>
    vector.insert r9:r10, r6:r7, r5, r8: vector<int32, 4>
    vector.shuffle r5:r6, r9:r10, r2:r3, [0, 5, 2, 7]: vector<int32, 4>
    vector.select r7:r8, r4, r5:r6, r0:r1: vector<int32, 4>
    vector.add r4:r5, r0:r1, r2:r3: vector<int32, 4>
    vector.reduce.add r0, r4:r5: vector<int32, 4>
    vector.compare.lt r1, r7:r8, r2:r3: vector<int32, 4>
    vector.convert.exact r1:r2, r7:r8: vector<int32, 4> -> vector<int32, 4>
    return r0
}
"#,
    );
}
