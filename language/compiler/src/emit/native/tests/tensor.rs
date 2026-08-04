use crate::tests::TestProgram;

/// Lower tensor allocation and element access through compact runtime commands.
#[test]
fn test_emit_native_tensor_access() {
    let program = TestProgram::mir(
        r#"
export function access(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: tensor<int32, managed, mutable, (2, 2)> = tensor.splat v0
    v4: int32 = tensor.extract v3, [v1, v2]
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    ss0 = explicit_slot 16, align = 8
    ss1 = explicit_slot 32, align = 8
    gv0 = symbol colocated userextname0
    gv1 = symbol colocated userextname1
    sig0 = (i64, i64, i64, i64) native
    sig1 = (i64, i64, i64, i64) native

block0(v0: i64, v1: i32):
    v2 = iconst.i32 0
    v3 = iconst.i32 1
    v4 = symbol_value.i64 gv0
    v5 = stack_addr.i64 ss0
    v6 = sextend.i64 v1
    store notrap aligned v6, v5
    v7 = iconst.i64 2
    v8 = load.i64 notrap aligned v0+8
    v9 = load.i64 notrap aligned v8+224
    call_indirect sig0, v9(v0, v4, v5, v7)  ; v7 = 2
    v10 = iconst.i64 8
    v11 = iadd v5, v10  ; v10 = 8
    v12 = load.i64 notrap aligned v11
    v13 = symbol_value.i64 gv1
    v14 = stack_addr.i64 ss1
    store notrap aligned v12, v14
    v15 = iconst.i64 8
    v16 = iadd v14, v15  ; v15 = 8
    v17 = sextend.i64 v2  ; v2 = 0
    store notrap aligned v17, v16
    v18 = iconst.i64 16
    v19 = iadd v14, v18  ; v18 = 16
    v20 = sextend.i64 v3  ; v3 = 1
    store notrap aligned v20, v19
    v21 = iconst.i64 4
    v22 = load.i64 notrap aligned v0+8
    v23 = load.i64 notrap aligned v22+224
    call_indirect sig1, v23(v0, v13, v14, v21)  ; v21 = 4
    v24 = iconst.i64 24
    v25 = iadd v14, v24  ; v24 = 24
    v26 = load.i32 notrap aligned v25
    return v26
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

/// Lower indexed tensor execution through one immutable linked descriptor.
#[test]
fn test_emit_native_tensor_gather() {
    let program = TestProgram::mir(
        r#"
export function gather(
    v0: tensor<int32, managed, mutable, (2, 2)>,
    v1: tensor<int32, managed, mutable, (1, 2)>,
): tensor<int32, managed, mutable, (1, 2)> {
entry(v0: tensor<int32, managed, mutable, (2, 2)>, v1: tensor<int32, managed, mutable, (1, 2)>):
    v2: tensor<int32, managed, mutable, (1, 2)> = tensor.gather v0, v1, dims(offsetDims(1), collapsedSliceDims(0), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 2)
    return v2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64) -> i64 native {
    ss0 = explicit_slot 24, align = 8
    gv0 = symbol colocated userextname0
    sig0 = (i64, i64, i64, i64) native

block0(v0: i64, v1: i64, v2: i64):
    v3 = symbol_value.i64 gv0
    v4 = stack_addr.i64 ss0
    store notrap aligned v1, v4
    v5 = iconst.i64 8
    v6 = iadd v4, v5  ; v5 = 8
    store notrap aligned v2, v6
    v7 = iconst.i64 3
    v8 = load.i64 notrap aligned v0+8
    v9 = load.i64 notrap aligned v8+224
    call_indirect sig0, v9(v0, v3, v4, v7)  ; v7 = 3
    v10 = iconst.i64 16
    v11 = iadd v4, v10  ; v10 = 16
    v12 = load.i64 notrap aligned v11
    return v12
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}
