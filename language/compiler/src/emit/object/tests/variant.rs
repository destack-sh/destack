use crate::tests::TestProgram;

/// Emit tagged variant construction, tag access, and payload projection.
#[test]
fn test_emit_variant() {
    let program = TestProgram::mir(
        r#"
export function payload(v0: int32): int32 {
entry(v0: int32):
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = boolean; } = variant.new 0, v0
    v2: uint1 = variant.tag v1
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
    extract r0, r1, 4:1
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    ss0 = explicit_slot 8, align = 4

block0(v0: i64, v1: i32):
    v2 = stack_addr.i64 ss0
    v3 = iconst.i64 0
    store notrap aligned v3, v2  ; v3 = 0
    v4 = iconst.i64 4
    v5 = iadd v2, v4  ; v4 = 4
    store notrap aligned v1, v5
    v6 = iconst.i64 0
    v7 = iadd v2, v6  ; v6 = 0
    v8 = load.i8 notrap aligned v7
    v9 = iconst.i8 -1
    v10 = bnot v9  ; v9 = -1
    v11 = band v8, v10
    v12 = iconst.i8 0
    v13 = bor v11, v12  ; v12 = 0
    v14 = iconst.i64 0
    v15 = iadd v2, v14  ; v14 = 0
    store notrap aligned v13, v15
    v16 = iconst.i64 0
    v17 = iadd v2, v16  ; v16 = 0
    v18 = load.i8 notrap aligned v17
    v19 = iconst.i8 -1
    v20 = band v18, v19  ; v19 = -1
    v21 = iconst.i8 0
    v22 = ushr v20, v21  ; v21 = 0
    v23 = iconst.i64 4
    v24 = iadd v2, v23  ; v23 = 4
    v25 = load.i32 notrap aligned v24
    return v25
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
